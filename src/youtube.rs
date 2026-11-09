pub mod domain;
pub mod id;

use anyhow::Context;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::youtube::id::YoutubeID;

pub fn probe() -> anyhow::Result<()> {
    let _: PathBuf = which::which("yt-dlp").context("Command yt-dlp not found")?;
    Ok(())
}

pub fn download(dir: &Path, videos: impl Iterator<Item = YoutubeID>) -> anyhow::Result<()> {
    debug_assert!(dir.is_dir());
    let mut cmd = Command::new("yt-dlp");
    _ = cmd.arg("-tmp3"); // template for mp3 (-x --audio-format=mp3) and some other shit
    _ = cmd.arg("-N4"); // concurrent downloads
    _ = cmd.arg("--write-thumbnail");
    _ = cmd.arg("--convert-thumbnails=png"); //TODO: verify
    _ = cmd.arg("-P");
    _ = cmd.arg(dir);
    _ = cmd.arg("-o%(id)s.%(ext)s");
    _ = cmd.arg("--");
    for video_id in videos {
        _ = cmd.arg(video_id.as_str());
    }

    let status = cmd.status().context("failed to execute command")?;
    anyhow::ensure!(status.success(), "yt-dlp failed; see its logs above");
    Ok(())
}
