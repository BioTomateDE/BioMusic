use std::{
    any::type_name,
    collections::BTreeMap,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{Serialize, de::DeserializeOwned};

use crate::{decl::ResourceType, ident::Ident};

pub fn read_toml_dir<T: ResourceType>(root_dir: &Path) -> anyhow::Result<BTreeMap<Ident, T>> {
    let mut map = BTreeMap::new();

    for path in listdir(root_dir.join(T::DIRECTORY), "toml") {
        let ident: Ident = ident_from_path(&path)?;
        let toml_struct: T = read_toml_file(&path)?;
        assert_eq!(map.insert(ident, toml_struct), None);
    }

    Ok(map)
}

pub fn ident_from_path(path: &Path) -> anyhow::Result<Ident> {
    path.file_stem()
        .context("Filename missing somehow")?
        .to_str()
        .context("Non UTF-8 filename")?
        .to_owned()
        .try_into()
        .context("Filename is not a valid identifier")
}

pub fn read_toml_file<T: DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    let ctx = || format!("Could not read {} toml file `{}`", type_name::<T>(), path.display());
    let file_text: String = std::fs::read_to_string(path)
        .context("Could not read toml file")
        .with_context(ctx)?;
    let toml_struct: T = ::toml::from_str(&file_text)
        .context("Could not deserialize toml")
        .with_context(ctx)?;
    Ok(toml_struct)
}

pub fn write_toml_dir<T: ResourceType>(map: &BTreeMap<Ident, T>, root_dir: &Path) -> anyhow::Result<()> {
    for (ident, toml_struct) in map {
        write_toml_resource(toml_struct, ident, root_dir)?;
    }
    Ok(())
}

pub fn write_toml_resource<T: ResourceType>(toml_struct: &T, ident: &Ident, root_dir: &Path) -> anyhow::Result<()> {
    write_toml_file(toml_struct, ident, &root_dir.join(T::DIRECTORY))
}

pub fn write_toml_file<T: Serialize>(toml_struct: &T, ident: &Ident, dir_path: &Path) -> anyhow::Result<()> {
    let path = dir_path.join(ident.string()).with_extension("toml");
    eprintln!(
        "\x1B[2m(Writing {} toml file `{}`)\x1B[0m",
        type_name::<T>(),
        path.display()
    );
    // TODO: dotted
    let toml_text = ::toml::to_string(toml_struct).context("failed to serialize toml")?;
    std::fs::write(&path, toml_text)
        .with_context(|| format!("failed to write {} toml file `{}`", type_name::<T>(), path.display()))
}

pub fn listdir(dir_path: impl AsRef<Path>, extension: impl AsRef<OsStr>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let readdir = std::fs::read_dir(dir_path).expect("could not read directory");
    for entry in readdir {
        let entry = entry.expect("could not read directory entry");
        if entry.file_name().as_encoded_bytes().starts_with(b".") {
            continue;
        }
        let path = entry.path();
        let Some(ext) = path.extension() else {
            continue;
        };
        if !ext.eq_ignore_ascii_case(&extension) {
            continue;
        }
        paths.push(path);
    }
    paths.sort();
    paths
}

pub fn ensure_dir(dir_path: impl AsRef<Path>) -> anyhow::Result<()> {
    let dir_path: &Path = dir_path.as_ref();
    match std::fs::create_dir(dir_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e).context(format!("could not create directory `{}`", dir_path.display())),
    }
}
