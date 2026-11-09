use serde::{Deserialize, Serialize};

use crate::list::List;

pub type Artist = super::Resource<ArtistDecl>;
impl super::ResourceType for ArtistDecl {
    const DIRECTORY: &str = "artists";
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtistDecl {
    pub name: String,

    #[serde(skip_serializing_if = "List::is_empty")]
    pub aka: List<String>,

    pub socials: Option<Socials>,
}

// TODO: more socials, handle/id validation
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Socials {
    /// <https://vocaloidlyrics.miraheze.org>
    ///
    /// Format: `https://vocaloidlyrics.miraheze.org/wiki/{}`
    pub wiki: String,

    /// <https://youtube.com>
    ///
    /// Format: `https://youtube.com/@{}`
    pub youtube: String,

    /// <https://spotify.com>
    ///
    /// Format: `https://open.spotify.com/artist/{}`
    /// This is an 22-char-long (probably) Base62 (idk) ID.
    pub spotify: Option<String>,

    /// <https://soundcloud.com>
    ///
    /// Format: `https://soundcloud.com/{}`
    pub soundcloud: Option<String>,

    /// <https://twitter.com>
    ///
    /// Format: `https://twitter.com/{}`
    pub twitter: Option<String>,
}
