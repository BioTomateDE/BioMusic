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
        Ok(Audio {
            samples,
            sample_rate,
            channels,
        })
    }

    /// Number of sample frames (one frame = one sample per channel).
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels.max(1)
    }

    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.frames() as f64 / self.sample_rate as f64)
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

    /// Linear fade in / fade out, useful to avoid clicks at the trim points.
    pub fn fade(&mut self, fade_in: Duration, fade_out: Duration) {
        let frames = self.frames();
        let ch = self.channels;
        let n_in = ((fade_in.as_secs_f64() * self.sample_rate as f64) as usize).min(frames);
        let n_out = ((fade_out.as_secs_f64() * self.sample_rate as f64) as usize).min(frames);

        for f in 0..n_in {
            let g = f as f32 / n_in as f32;
            for s in &mut self.samples[f * ch..(f + 1) * ch] {
                *s *= g;
            }
        }
        for i in 0..n_out {
            // Last frame gets gain ~0, frame `frames - n_out` gets gain ~1.
            let f = frames - 1 - i;
            let g = i as f32 / n_out as f32;
            for s in &mut self.samples[f * ch..(f + 1) * ch] {
                *s *= g;
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn sine(sample_rate: u32, channels: usize, secs: f32, amp: f32) -> Audio {
        let frames = (sample_rate as f32 * secs) as usize;
        let mut samples = Vec::with_capacity(frames * channels);
        for i in 0..frames {
            let v = amp * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sample_rate as f32).sin();
            for _ in 0..channels {
                samples.push(v);
            }
        }
        Audio {
            samples,
            sample_rate,
            channels,
        }
    }

    #[test]
    fn roundtrip_trim_normalize() {
        for channels in [1, 2] {
            let src = sine(44_100, channels, 5.0, 0.25);
            let mp3 = src.encode_mp3(Bitrate::Kbps192).unwrap();

            let mut audio = Audio::decode(Cursor::new(mp3), Some("mp3")).unwrap();
            assert_eq!(audio.sample_rate, 44_100);
            assert_eq!(audio.channels, channels);

            audio.trim(Some(Duration::from_secs(1)), Some(Duration::from_secs(3)));
            let secs = audio.duration().as_secs_f32();
            assert!((secs - 2.0).abs() < 0.01, "duration {secs}");

            audio.normalize_peak(-1.0);
            let target = 10f32.powf(-1.0 / 20.0);
            assert!((audio.peak() - target).abs() < 1e-4, "peak {}", audio.peak());
        }
    }

    #[test]
    fn trim_clamps_and_keeps_frames_aligned() {
        let mut a = sine(8_000, 2, 1.0, 0.5);
        a.trim(Some(Duration::from_millis(500)), Some(Duration::from_secs(99)));
        assert_eq!(a.samples.len() % 2, 0);
        assert_eq!(a.frames(), 4_000);

        a.trim(Some(Duration::from_secs(10)), None); // start past the end -> empty
        assert!(a.samples.is_empty());
    }

    #[test]
    fn silence_is_not_normalized() {
        let mut a = Audio {
            samples: vec![0.0; 100],
            sample_rate: 8_000,
            channels: 1,
        };
        a.normalize_peak(-1.0);
        assert!(a.samples.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn fade_hits_zero_at_edges() {
        let mut a = Audio {
            samples: vec![1.0; 2 * 1000],
            sample_rate: 1000,
            channels: 2,
        };
        a.fade(Duration::from_millis(100), Duration::from_millis(100));
        assert_eq!(a.samples[0], 0.0);
        assert!(a.samples[a.samples.len() - 1] < 0.02);
        assert_eq!(a.samples[1000], 1.0); // middle untouched
    }

    #[test]
    fn errors_are_descriptive() {
        let err = Audio::open("/definitely/not/here.mp3").unwrap_err();
        assert!(format!("{err:#}").contains("failed to open"));

        let err = Audio::decode(Cursor::new(vec![0u8; 64]), Some("mp3")).unwrap_err();
        assert!(!format!("{err:#}").is_empty());

        let a = Audio {
            samples: vec![0.0; 6],
            sample_rate: 8_000,
            channels: 3,
        };
        assert!(
            a.encode_mp3(Bitrate::Kbps128)
                .unwrap_err()
                .to_string()
                .contains("mono or stereo")
        );
    }
}
