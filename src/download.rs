use crate::{
    BioMusic, cachedir,
    decl::song::{SongDecl, SongSource},
    io::ensure_dir,
    youtube::id::YoutubeID,
};
use anyhow::Context;
use id3::TagLike;
use std::path::{Path, PathBuf};

pub fn download_missing(music: &BioMusic) -> anyhow::Result<()> {
    let dir = cachedir()?;
    let songs_dir = dir.join("songs");
    ensure_dir(&songs_dir)?;

    let yt_cache_dir = dir.join("youtube");
    ensure_dir(&yt_cache_dir)?;

    let mut videos: Vec<(&SongDecl, PathBuf)> = Vec::with_capacity(music.songs.len());

    for (name, song) in &music.songs {
        let path = songs_dir.join(name.string()).with_extension("mp3");
        if path.exists() {
            continue;
        }

        let youtube_id = useless(song);

        // check for cached youtube download
        if video_cache_path(&yt_cache_dir, youtube_id).is_file() {
            dostuff(song, &yt_cache_dir, &path, &music.root_dir_path)?;
            continue;
        }

        videos.push((song, path));
    }

    if videos.is_empty() {
        return Ok(());
    }

    crate::youtube::probe()?;

    println!("Downloading {} YouTube videos", videos.len());
    // btw this might break for case-insensitive filesystems
    crate::youtube::download(&yt_cache_dir, videos.iter().map(|x| useless(x.0)))
        .context("Could not download videos from youtube")?;

    for (song, target_path) in videos {
        dostuff(song, &yt_cache_dir, &target_path, &music.root_dir_path)?;
    }

    Ok(())
}

const fn useless(song: &SongDecl) -> YoutubeID {
    match song.source {
        SongSource::Youtube(youtube_id) => youtube_id,
    }
}

fn video_cache_path(cache_dir: &Path, id: YoutubeID) -> PathBuf {
    cache_dir.join(id.as_str()).with_extension("mp3")
}

fn dostuff(song: &SongDecl, source_dir: &Path, target_path: &Path, root_dir: &Path) -> anyhow::Result<()> {
    let id = useless(song);
    let artist = song.artist.resolve(root_dir)?;
    let image_path: PathBuf = source_dir.join(id.as_str()).with_extension("png");
    let image_data: Vec<u8> = std::fs::read(image_path).context("Could not read thumbnail image")?;

    let mut tag = id3::Tag::new();
    tag.set_title(&song.title);
    tag.set_artist(&artist.name);
    _ = tag.add_frame(id3::frame::Picture {
        mime_type: "image/png".to_owned(),
        picture_type: id3::frame::PictureType::CoverFront,
        description: String::new(),
        data: image_data,
    });

    // No way to write id3 data to new/copied mp3 file it seems.
    // To make sure the id3 data is always written to the target file,
    // write it to the source cache file first.
    let source_path = video_cache_path(source_dir, id);
    tag.write_to_path(&source_path, id3::Version::Id3v24)
        .context("Could not write id3 data")?;

    crate::normalize_audio::process_file(source_path, target_path, song.trim.start, song.trim.end)
        .context("Could not trim and normalize audio")?;

    // std::fs::rename(source_path, target_path).context("Could not move temp download file")?;
    Ok(())
}
