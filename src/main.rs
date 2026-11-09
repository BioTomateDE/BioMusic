mod add;
mod decl;
mod download;
mod ident;
mod io;
mod list;
mod normalize_audio;
mod serde_duration;
mod validate;
mod youtube;

use crate::{
    decl::{artist::ArtistDecl, character::CharacterDecl, game::GameDecl, song::SongDecl},
    ident::Ident,
};
use anyhow::Context;
use clap::{CommandFactory, Parser, Subcommand};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Parser)]
struct Args {
    /// Biomusic instance directory; defaults to ~/.biomusic
    #[arg(long, short)]
    dir: Option<PathBuf>,

    /// Skips validation on references
    #[arg(long)]
    no_validate: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Format the TOML files
    Format,

    /// Download missing audio files
    Download,

    /// Clear everything from the cache
    Clear,

    /// Add a new song or other resource
    Add(AddArgs),
}

#[derive(Debug, Parser)]
struct AddArgs {
    #[command(subcommand)]
    resource: AddResource,
}

#[derive(Debug, Subcommand)]
enum AddResource {
    Song,
    Artist,
    // Character,
    // Game,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BioMusic {
    root_dir_path: PathBuf,
    games: BTreeMap<Ident, GameDecl>,
    characters: BTreeMap<Ident, CharacterDecl>,
    artists: BTreeMap<Ident, ArtistDecl>,
    songs: BTreeMap<Ident, SongDecl>,
}

impl BioMusic {
    fn read_dir(root_dir_path: PathBuf) -> anyhow::Result<Self> {
        let root = &root_dir_path;
        let games = io::read_toml_dir(root)?;
        let characters = io::read_toml_dir(root)?;
        let artists = io::read_toml_dir(root)?;
        let songs = io::read_toml_dir(root)?;
        Ok(Self {
            root_dir_path,
            games,
            characters,
            artists,
            songs,
        })
    }

    fn write_dir(&self) -> anyhow::Result<()> {
        let root = &self.root_dir_path;
        io::write_toml_dir(&self.games, root)?;
        io::write_toml_dir(&self.characters, root)?;
        io::write_toml_dir(&self.artists, root)?;
        io::write_toml_dir(&self.songs, root)?;
        Ok(())
    }
}

fn report_error(error: &anyhow::Error) -> String {
    let mut buffer = error.to_string();
    for e in error.chain().skip(1) {
        buffer += "\n> ";
        buffer += &e.to_string();
    }
    buffer
}

fn main() {
    let args = Args::parse();
    if let Err(e) = those_who_snow(args) {
        Args::command()
            .error(::clap::error::ErrorKind::ValueValidation, report_error(&e))
            .exit();
    }
}

fn those_who_snow(args: Args) -> anyhow::Result<()> {
    let root = if let Some(dir) = args.dir {
        dir
    } else {
        let home = std::env::home_dir().context("failed to get home dir")?;
        home.join(".biomusic")
    };
    anyhow::ensure!(root.is_dir(), "instance dir missing");

    let mut music = BioMusic::read_dir(root)?;

    let validate = !args.no_validate;

    match args.command {
        Command::Format => {
            music.write_dir().context("could not format toml files")?;
            println!("Formatted!");
            if validate {
                validate::validate(&music)?;
            }
        }
        Command::Add(subargs) => {
            add::add_resource(subargs, &mut music)?;
        }
        Command::Download => {
            if validate {
                validate::validate(&music)?;
            }
            download::download_missing(&music)?;
            println!("Downloads complete!");
        }
        Command::Clear => {
            let dir = cachedir()?;
            std::fs::remove_dir_all(dir).context("failed to remove cache dir")?;
            println!("Cache cleared!");
            if validate {
                validate::validate(&music)?;
            }
        }
    }

    Ok(())
}

fn cachedir() -> anyhow::Result<PathBuf> {
    let home = std::env::home_dir().context("failed to get home dir")?;
    let dir = home.join(".cache");
    anyhow::ensure!(dir.is_dir(), "missing ~/.cache dir");
    let dir = dir.join("biomusic");
    io::ensure_dir(&dir)?;
    Ok(dir)
}
