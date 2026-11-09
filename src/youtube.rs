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
    let home_dir = std::env::home_dir().context("Could not get home dir")?;

    let mut cmd = Command::new("yt-dlp");
    _ = cmd.arg("-tmp3"); // template for mp3 (-x --audio-format=mp3) and some other shit
    _ = cmd.arg("-N4"); // concurrent downloads
    _ = cmd.arg("--write-thumbnail");
    if let Some(browser) = find_browser_cookies(&home_dir) {
        println!("Using browser cookies from {browser}");
        _ = cmd.arg("--cookies-from-browser");
        _ = cmd.arg(browser);
    } else {
        println!("No browser cookies found, may lead to rate-limiting");
    }
    _ = cmd.arg("--convert-thumbnails=png"); //TODO: verify
    _ = cmd.arg("-P");
    _ = cmd.arg(dir);
    _ = cmd.arg("-o%(id)s.%(ext)s");
    _ = cmd.arg("--");
    for video_id in videos {
        _ = cmd.arg(video_id.as_str());
    }

    println!("=============== yt-dlp logs ===============");
    let status = cmd.status().context("Failed to execute yt-dlp command")?;
    anyhow::ensure!(status.success(), "yt-dlp failed; see its logs above");
    println!();
    Ok(())
}

fn find_browser_cookies(home_dir: &Path) -> Option<&'static str> {
    // These are the paths that yt-dlp looks for.
    // Als see docs for `--cookies-from-browser`.
    const BROWSERS: &[(&str, &str)] = &[
        ("firefox", ".config/mozilla/firefox"),
        ("firefox", ".mozilla/firefox"),
        ("firefox", ".var/app/org.mozilla.firefox/config/mozilla/firefox"),
        ("firefox", ".var/app/org.mozilla.firefox/.mozilla/firefox"),
        ("firefox", "snap/firefox/common/.mozilla/firefox"),
        ("vivaldi", ".config/vivaldi"),
        ("brave", ".config/BraveSoftware/Brave-Browser"),
        ("whale", ".config/naver-whale"),
        ("chromium", ".config/chromium"),
        ("chromium", ".config/chromium"),
        ("chrome", ".config/google-chrome"),
        ("opera", ".config/opera"),
        ("edge", ".config/microsoft-edge"),
    ];

    for (browser, path) in BROWSERS {
        if home_dir.join(path).is_dir() {
            return Some(browser);
        }
    }

    None
}
