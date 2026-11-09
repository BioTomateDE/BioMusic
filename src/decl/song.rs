use std::time::Duration;

use crate::{
    decl::{artist::Artist, character::Character, game::Game},
    list::List,
    youtube::id::YoutubeID,
};
use serde::{Deserialize, Serialize};

// pub type Lang = ::iso::language::Iso639_1;
pub type Lang = ::iso639_1::Iso639_1;

pub type Song = super::Resource<SongDecl>;
impl super::ResourceType for SongDecl {
    const DIRECTORY: &str = "songs";
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SongDecl {
    pub source: SongSource,

    pub title: String,

    pub artist: Artist,

    #[serde(skip_serializing_if = "List::is_empty")]
    pub feat: List<Artist>,

    // TODO: allow more than 2 songs
    pub mashup: Option<Box<[Song; 2]>>,

    pub remix: Option<Box<Song>>,

    #[serde(rename = "mod")]
    pub modif: Option<Modification>,

    #[serde(skip_serializing_if = "List::is_empty")]
    pub char: List<Character>,

    #[serde(skip_serializing_if = "List::is_empty")]
    pub mood: List<Mood>,

    pub game: Option<Game>,

    #[serde(skip_serializing_if = "List::is_empty")]
    pub lang: List<Lang>,

    #[serde(skip_serializing_if = "List::is_empty")]
    pub aka: List<String>,

    #[serde(default, skip_serializing_if = "AudioTrim::is_none")]
    pub trim: AudioTrim,
}

impl SongDecl {
    pub const fn new(source: SongSource, title: String, artist: Artist) -> Self {
        Self {
            source,
            title,
            artist,
            feat: List::EMPTY,
            mashup: None,
            remix: None,
            modif: None,
            char: List::EMPTY,
            mood: List::EMPTY,
            game: None,
            lang: List::EMPTY,
            aka: List::EMPTY,
            trim: AudioTrim::NONE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mood {
    Chill,
    Kawaii,
    Sad,
    Upbeat,
    Schizo,
    Power,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SongSource {
    Youtube(YoutubeID),
}

// TODO: implement. also time for string
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AudioTrim {
    #[serde(default, with = "crate::serde_duration", skip_serializing_if = "Option::is_none")]
    pub start: Option<Duration>,

    #[serde(default, with = "crate::serde_duration", skip_serializing_if = "Option::is_none")]
    pub end: Option<Duration>,
}

impl AudioTrim {
    pub const NONE: Self = Self::new(None, None);

    pub const fn new(start: Option<Duration>, end: Option<Duration>) -> Self {
        Self { start, end }
    }
    pub fn is_none(&self) -> bool {
        *self == Self::NONE
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Modification {
    Nightcore,
}
