use crate::BioMusic;
use crate::decl::song::SongDecl;
use crate::report_error;
use std::path::Path;

pub fn validate(music: &BioMusic) -> anyhow::Result<()> {
    let root = &music.root_dir_path;
    let mut fail_count: usize = 0;
    let total_count = music.songs.len();

    for (name, song) in &music.songs {
        let Err(e) = validate_song(song, root) else {
            continue;
        };
        let e = e.context(format!("Invalid references in song declaration `{name}`"));
        eprintln!("{}", report_error(&e));
        fail_count += 1;
    }

    if fail_count > 0 {
        anyhow::bail!("Validation failed for {fail_count}/{total_count} songs");
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
