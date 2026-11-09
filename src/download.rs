use crate::{
    BioMusic, cachedir,
    decl::song::{SongDecl, SongSource},
    io::ensure_dir,
    songdir,
    youtube::id::YoutubeID,
};
use anyhow::Context;
use id3::TagLike;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{
    fs::File,
    io::Seek,
    path::{Path, PathBuf},
};

pub fn download_missing(music: &BioMusic) -> anyhow::Result<()> {
    let songs_dir = songdir()?;
    ensure_dir(&songs_dir)?;

    let yt_cache_dir = cachedir()?.join("youtube");
    ensure_dir(&yt_cache_dir)?;

    let mut videos: Vec<YoutubeID> = Vec::with_capacity(music.songs.len());
    let mut songs: Vec<(&SongDecl, PathBuf)> = Vec::with_capacity(music.songs.len());

    for (name, song) in &music.songs {
        let path = songs_dir.join(name.string()).with_extension("mp3");
        if path.exists() {
            continue;
        }

        let youtube_id = useless(song);

        // check for cached youtube download
        if !video_cache_path(&yt_cache_dir, youtube_id).is_file() {
            videos.push(youtube_id);
        }
        songs.push((song, path));
    }

    if !videos.is_empty() {
        crate::youtube::probe()?;
        println!("Downloading {} YouTube videos", videos.len());
        // btw this might break for case-insensitive filesystems
        crate::youtube::download(&yt_cache_dir, &videos).context("Could not download videos from youtube")?;
    }

    if songs.is_empty() {
        return Ok(());
    }

    songs
        .into_par_iter()
        .try_for_each(|(song, target_path)| dostuff(song, &yt_cache_dir, &target_path, &music.root_dir_path))?;

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
    println!("Creating audio file for song {:?} from YouTube ID {id}", song.title);
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

    let dir = target_path.parent().unwrap();
    let mut tempfile = tempfile::tempfile_in(dir).context("Could not create tempfile")?;

    let source_path = video_cache_path(source_dir, id);
    // TODO: this is slow and garbage
    crate::normalize_audio::process_file(source_path, &mut tempfile, song.trim.start, song.trim.end)
        .context("Could not trim and normalize audio")?;

    tag.write_to_file(&mut tempfile, id3::Version::Id3v24)
        .context("Could not write id3 data")?;

    // This copy is probably needed if you don't want a NamedTempFile (which would be less stable)
    let mut target_file = File::create_new(target_path).context("Could not open target file")?;
    tempfile.rewind().context("Could not rewind tempfile cursor")?;
    _ = std::io::copy(&mut tempfile, &mut target_file).context("Could not copy tempfile to target file")?;
    Ok(())
}
