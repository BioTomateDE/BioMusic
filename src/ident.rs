use std::fmt;

use serde::{Deserialize, Serialize};

/// A kebab-case identifier for a song, artist or similar.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(try_from = "String")]
pub struct Ident(String);

impl Ident {
    pub const fn string(&self) -> &String {
        &self.0
    }

    // pub fn into_string(self) -> String {
    //     self.0
    // }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Ident {
    type Error = IdentParseError;

    fn try_from(string: String) -> Result<Self, Self::Error> {
        if string.is_empty() {
            return Err(IdentParseError::Empty);
        }
        if string.starts_with('-') {
            return Err(IdentParseError::LeadingDash);
        }
        if string.ends_with('-') {
            return Err(IdentParseError::TrailingDash);
        }
        for (idx, ch) in string.char_indices() {
            if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' {
                continue;
            }
            return Err(if ch.is_ascii_uppercase() {
                IdentParseError::Uppercase(ch, idx)
            } else {
                IdentParseError::InvalidChar(ch, idx)
            });
        }
        if let Some(idx) = string.find("--") {
            return Err(IdentParseError::MultiDash(idx));
        }
        Ok(Self(string))
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum IdentParseError {
    #[error("Identifier is empty")]
    Empty,

    #[error("Identifier contains uppercase character {0:?} at index {1}")]
    Uppercase(char, usize),

    #[error("Identifier contains invalid character {0:?} at index {1}")]
    InvalidChar(char, usize),

    #[error("Identifier starts with a dash")]
    LeadingDash,

    #[error("Identifier ends with a dash")]
    TrailingDash,

    #[error("Identifier contains multiple dashes in a row at index {0}")]
    MultiDash(usize),
}
