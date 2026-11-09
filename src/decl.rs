use crate::ident::Ident;
use anyhow::Context;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    any::type_name,
    borrow::Cow,
    fmt::Debug,
    io::ErrorKind,
    path::{Path, PathBuf},
};

pub mod artist;
pub mod character;
pub mod game;
pub mod song;

pub trait ResourceType: DeserializeOwned + Serialize + Clone + Debug + PartialEq + Eq {
    const DIRECTORY: &str;
}

/// A reference to a resource `T` using an identifier, or an inline declaration of `T`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Resource<T: ResourceType> {
    /// Reference to a resource `T` defined in another file.
    Ref(Ident),

    /// Declaration right here.
    Decl(T),
}

impl<T: ResourceType> Resource<T> {
    pub fn resolve<'a>(&'a self, root_dir: &Path) -> anyhow::Result<Cow<'a, T>> {
        let ident = match self {
            Self::Ref(ident) => ident,
            Self::Decl(decl) => return Ok(Cow::Borrowed(decl)),
        };
        let path = get_path::<T>(ident, root_dir);
        let decl: T = crate::io::read_toml_file(&path).context("reading toml file")?;
        Ok(Cow::Owned(decl))
    }

    pub fn validate(&self, root_dir: &Path) -> anyhow::Result<()> {
        let Self::Ref(ident) = self else { return Ok(()) };
        validate_resource::<T>(ident, root_dir)
    }
}

pub fn validate_resource<T: ResourceType>(ident: &Ident, root_dir: &Path) -> anyhow::Result<()> {
    let path = get_path::<T>(ident, root_dir);
    let Err(e) = path.metadata() else { return Ok(()) };
    if e.kind() == ErrorKind::NotFound && path.parent().unwrap().exists() {
        anyhow::bail!("Dangling {} resource reference `{ident}`", type_name::<T>());
    }
    // some weird error; actually print it
    Err(anyhow::Error::new(e).context(format!(
        "Could not resolve {} resource reference at `{}`",
        type_name::<T>(),
        path.display(),
    )))
}

fn get_path<T: ResourceType>(ident: &Ident, root_dir: &Path) -> PathBuf {
    root_dir.join(T::DIRECTORY).join(ident.string()).with_extension("toml")
}
