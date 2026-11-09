use crate::{BioMusic, decl::song::SongDecl};
use anyhow::Context;
use std::path::Path;

pub fn validate(music: &BioMusic) -> anyhow::Result<()> {
    let root = &music.root_dir_path;

    for (name, song) in &music.songs {
        validate_song(song, root).with_context(|| format!("Invalid references in song declaration `{name}`"))?;
    }

    Ok(())
}

fn validate_song(song: &SongDecl, root: &Path) -> anyhow::Result<()> {
    song.artist.validate(root)?;
    for artist in &song.feat.0 {
        artist.validate(root)?;
    }
    if let Some(songs) = &song.mashup {
        for song in songs {
            song.validate(root)?;
        }
    }
    if let Some(song) = &song.remix {
        song.validate(root)?;
    }
    for character in &song.char.0 {
        character.validate(root)?;
    }
    if let Some(game) = &song.game {
        game.validate(root)?;
    }
    Ok(())
}
