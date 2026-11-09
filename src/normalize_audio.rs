//! ai slop
//! Decode an MP3, trim it, normalize its volume and re-encode it.
//!
//! Decoding uses `symphonia` 0.6, encoding uses `mp3lame-encoder` (LAME).
//! All edits happen on interleaved `f32` samples in the range `-1.0..=1.0`.
//! Errors are reported with `anyhow`.
//!
//! ```ignore
//! use std::time::Duration;
//! use crate::normalize_audio::{Audio, Bitrate};
//!
//! let mut audio = Audio::open("in.mp3")?;
//! audio.trim(Duration::from_secs(2), Some(Duration::from_secs(30)));
//! audio.fade(Duration::from_millis(10), Duration::from_millis(10));
//! audio.normalize_peak(-1.0);
//! audio.write_mp3("out.mp3", Bitrate::Kbps192)?;
//! # anyhow::Ok(())
//! ```

use std::fmt::Debug;
use std::fs::File;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use mp3lame_encoder::{Builder, FlushNoGap, InterleavedPcm, MonoPcm, Quality};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;

pub use mp3lame_encoder::Bitrate;

/// LAME's error types don't implement `std::error::Error`, so wrap them by hand.
fn lame<E: Debug>(what: &'static str) -> impl FnOnce(E) -> anyhow::Error {
    move |e| anyhow!("{what}: {e:?}")
}

/// Decoded audio: interleaved `f32` samples (`L R L R ...` for stereo).
#[derive(Debug, Clone)]
pub struct Audio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: usize,
}

impl Audio {
    /// Decode an audio file from disk.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path).with_context(|| format!("Could not open audio file `{}`", path.display()))?;
        let ext = path.extension().and_then(|e| e.to_str());
        Self::decode(file, ext).with_context(|| format!("Could not decode audio at `{}`", path.display()))
    }

    /// Decode from any seekable source (a `File`, a `Cursor<Vec<u8>>`, ...).
    /// `extension` is an optional format hint such as `"mp3"`.
    pub fn decode<R: MediaSource + 'static>(source: R, extension: Option<&str>) -> Result<Self> {
        let mss = MediaSourceStream::new(Box::new(source), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = extension {
            _ = hint.with_extension(ext);
        }

        let mut format = symphonia::default::get_probe()
            .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
            .context("unrecognized or unsupported audio format")?;

        let track = format.default_track(TrackType::Audio).context("no audio track found")?;
        let track_id = track.id;
        let params = track
            .codec_params
            .as_ref()
            .and_then(|p| p.audio())
            .context("audio track has no usable codec parameters")?;
        let mut decoder = symphonia::default::get_codecs()
            .make_audio_decoder(params, &AudioDecoderOptions::default())
            .context("unsupported audio codec")?;

        let mut samples: Vec<f32> = Vec::new();
        let mut scratch: Vec<f32> = Vec::new();
        let mut spec: Option<(u32, usize)> = None;

        while let Some(packet) = format.next_packet().context("failed to read packet")? {
            if packet.track_id != track_id {
                continue;
            }
            match decoder.decode(&packet) {
                Ok(buf) => {
                    let fuck = (buf.spec().rate(), buf.spec().channels().count());
                    _ = spec.get_or_insert(fuck);
                    scratch.resize(buf.samples_interleaved(), 0.0);
                    buf.copy_to_slice_interleaved(scratch.as_mut_slice());
                    samples.extend_from_slice(&scratch);
                }
                // A corrupt packet: skip it and keep going.
                Err(SymphoniaError::DecodeError(_)) => {}
                Err(e) => return Err(e).context("failed to decode packet"),
            }
        }

        let (sample_rate, channels) = spec.context("file contained no decodable audio")?;
        Ok(Self {
            samples,
            sample_rate,
            channels,
        })
    }

    /// Number of sample frames (one frame = one sample per channel).
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels.max(1)
    }

    /// Keep only `start..end` (`end = None` keeps everything until the end).
    /// Out-of-range values are clamped. Cuts always fall on frame boundaries,
    /// so channels never get shifted.
    pub fn trim(&mut self, start: Option<Duration>, end: Option<Duration>) {
        let frames = self.frames();
        let to_frame = |d: Duration| ((d.as_secs_f64() * self.sample_rate as f64) as usize).min(frames);

        let start_f = to_frame(start.unwrap_or(Duration::ZERO));
        let end_f = end.map_or(frames, to_frame).max(start_f);

        let ch = self.channels;
        self.samples.truncate(end_f * ch);
        _ = self.samples.drain(..start_f * ch);
    }

    /// Highest absolute sample value (0.0 for silence).
    pub fn peak(&self) -> f32 {
        self.samples.iter().fold(0.0f32, |m, &s| m.max(s.abs()))
    }

    /// Scale so the loudest sample sits at `target_db` dBFS (e.g. `-1.0`).
    /// Silent audio is left untouched.
    pub fn normalize_peak(&mut self, target_db: f32) {
        let peak = self.peak();
        if peak <= f32::EPSILON {
            return;
        }
        let gain = 10f32.powf(target_db / 20.0) / peak;
        for s in &mut self.samples {
            *s *= gain;
        }
    }

    /// Encode to MP3 bytes (CBR at `bitrate`).
    pub fn encode_mp3(&self, bitrate: Bitrate) -> Result<Vec<u8>> {
        if self.channels != 1 && self.channels != 2 {
            bail!("MP3 encoding supports mono or stereo, got {} channels", self.channels);
        }

        let mut builder = Builder::new().context("failed to initialize LAME")?;
        builder
            .set_num_channels(self.channels as u8)
            .map_err(lame("invalid channel count"))?;
        builder
            .set_sample_rate(self.sample_rate)
            .map_err(lame("invalid sample rate"))?;
        builder.set_brate(bitrate).map_err(lame("invalid bitrate"))?;
        builder.set_quality(Quality::Good).map_err(lame("invalid quality"))?;
        let mut encoder = builder.build().map_err(lame("failed to build encoder"))?;

        let pcm: Vec<i16> = self
            .samples
            .iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16)
            .collect();

        let mut out = Vec::with_capacity(mp3lame_encoder::max_required_buffer_size(pcm.len()));
        if self.channels == 1 {
            _ = encoder
                .encode_to_vec(MonoPcm(&pcm), &mut out)
                .map_err(lame("encoding failed"))?;
        } else {
            _ = encoder
                .encode_to_vec(InterleavedPcm(&pcm), &mut out)
                .map_err(lame("encoding failed"))?;
        }
        _ = encoder
            .flush_to_vec::<FlushNoGap>(&mut out)
            .map_err(lame("flushing encoder failed"))?;
        Ok(out)
    }

    pub fn write_mp3(&self, mut file: impl id3::StorageFile, bitrate: Bitrate) -> Result<()> {
        let bytes = self.encode_mp3(bitrate).context("Failed to encode mp3")?;
        _ = file.write(&bytes).context("Failed to write encoded MP3 bytes")?;
        Ok(())
    }
}

/// One-call convenience: decode `input`, trim to `start..end`, peak-normalize to
/// `target_db` dBFS and write `output` as 192 kbps MP3.
pub fn process_file(
    input: impl AsRef<Path>,
    output: impl id3::StorageFile,
    start: Option<Duration>,
    end: Option<Duration>,
) -> Result<()> {
    let mut audio = Audio::open(input)?;
    audio.trim(start, end); // trim first so removed audio doesn't affect the peak
    audio.normalize_peak(-1.0);
    audio.write_mp3(output, Bitrate::Kbps192)
}
