//! Warning: slopcoded
use serde::de::{Deserializer, Visitor};
use serde::{Deserialize, Serialize, Serializer};
use std::{fmt, marker::PhantomData};

/// One or multiple values in TOML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct List<T>(pub Vec<T>);

impl<T> Default for List<T> {
    fn default() -> Self {
        Self(Vec::default())
    }
}

impl<T> List<T> {
    pub const EMPTY: Self = Self(Vec::new());

    // pub const fn len(&self) -> usize {
    //     self.0.len()
    // }

    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T: Serialize> Serialize for List<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.0.as_slice() {
            [single] => single.serialize(s),
            all => all.serialize(s),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for List<T> {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_option(Ghost(PhantomData))
    }
}

struct Ghost<T>(PhantomData<T>);
impl<'de, T: Deserialize<'de>> Visitor<'de> for Ghost<T> {
    type Value = List<T>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a value or a list of values")
    }

    // missing key -> empty list
    fn visit_none<E>(self) -> Result<List<T>, E> {
        Ok(List(Vec::new()))
    }

    fn visit_some<D2: Deserializer<'de>>(self, de: D2) -> Result<List<T>, D2::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum OneOrMany<T> {
            One(T),
            Many(Vec<T>),
        }
        Ok(List(match OneOrMany::deserialize(de)? {
            OneOrMany::One(x) => vec![x],
            OneOrMany::Many(v) => v,
        }))
    }
}
