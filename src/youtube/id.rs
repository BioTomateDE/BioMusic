use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize, Serializer};

const LENGTH: usize = 11;

/// A Youtube video ID.
///
/// Consists of 11 URL-safe Base-64 characters.
#[expect(clippy::unsafe_derive_deserialize, reason = "i am better")]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(try_from = "&str")]
pub struct YoutubeID([u8; LENGTH]);

impl YoutubeID {
    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY: ASCII subset validated in FromStr
        unsafe { str::from_utf8_unchecked(&self.0) }
    }
}

impl TryFrom<&str> for YoutubeID {
    type Error = YoutubeIDParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl FromStr for YoutubeID {
    type Err = YoutubeIDParseError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let array: [u8; LENGTH] = match string.as_bytes().try_into() {
            Ok(arr) => arr,
            Err(_) => return Err(YoutubeIDParseError::BadLength(string.len())),
        };
        for ch in string.chars() {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                continue;
            }
            return Err(YoutubeIDParseError::BadChar(ch));
        }
        Ok(Self(array))
    }
}

impl Serialize for YoutubeID {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl fmt::Display for YoutubeID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for YoutubeID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("YoutubeID").field(&self.as_str()).finish()
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum YoutubeIDParseError {
    #[error("expected string length {LENGTH}, got {0}")]
    BadLength(usize),
    #[error("invalid character {0:?}")]
    BadChar(char),
}
