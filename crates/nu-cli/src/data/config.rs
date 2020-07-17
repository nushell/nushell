mod conf;
mod nuconfig;

#[cfg(test)]
pub mod tests;

pub(crate) use conf::Conf;
pub(crate) use nuconfig::NuConfig;

use crate::commands::from_toml::convert_toml_value_to_nu_value;
use crate::commands::to_toml::value_to_toml_value;
use crate::prelude::*;
use indexmap::IndexMap;
use log::trace;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, ShellTypeName, UntaggedValue, Value};
use nu_source::Tag;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

#[cfg(feature = "directories")]
pub fn config_path() -> Result<PathBuf, ShellError> {
    use directories::ProjectDirs;

    let dir = ProjectDirs::from("org", "nushell", "nu")
        .ok_or_else(|| ShellError::untagged_runtime_error("Couldn't find project directory"))?;
    let path = ProjectDirs::config_dir(&dir).to_owned();
    std::fs::create_dir_all(&path).map_err(|err| {
        ShellError::untagged_runtime_error(&format!("Couldn't create {} path:\n{}", "config", err))
    })?;

    Ok(path)
}

#[cfg(not(feature = "directories"))]
pub fn config_path() -> Result<PathBuf, ShellError> {
    // FIXME: unsure if this should be error or a simple default

    Ok(std::path::PathBuf::from("/"))
}

pub fn default_path() -> Result<PathBuf, ShellError> {
    default_path_for(&None)
}

pub fn default_path_for(file: &Option<PathBuf>) -> Result<PathBuf, ShellError> {
    let mut filename = config_path()?;
    let file: &Path = file
        .as_ref()
        .map(AsRef::as_ref)
        .unwrap_or_else(|| "config.toml".as_ref());
    filename.push(file);

    Ok(filename)
}

#[cfg(feature = "directories")]
pub fn user_data() -> Result<PathBuf, ShellError> {
    use directories::ProjectDirs;

    let dir = ProjectDirs::from("org", "nushell", "nu")
        .ok_or_else(|| ShellError::untagged_runtime_error("Couldn't find project directory"))?;
    let path = ProjectDirs::data_local_dir(&dir).to_owned();
    std::fs::create_dir_all(&path).map_err(|err| {
        ShellError::untagged_runtime_error(&format!(
            "Couldn't create {} path:\n{}",
            "user data", err
        ))
    })?;

    Ok(path)
}

#[cfg(not(feature = "directories"))]
pub fn user_data() -> Result<PathBuf, ShellError> {
    // FIXME: unsure if this should be error or a simple default

    Ok(std::path::PathBuf::from("/"))
}

pub fn read(
    tag: impl Into<Tag>,
    at: &Option<PathBuf>,
) -> Result<IndexMap<String, Value>, ShellError> {
    let filename = default_path()?;

    let filename = match at {
        None => filename,
        Some(ref file) => file.clone(),
    };

    if !filename.exists() && touch(&filename).is_err() {
        // If we can't create configs, let's just return an empty indexmap instead as we may be in
        // a readonly environment
        return Ok(IndexMap::new());
    }

    trace!("config file = {}", filename.display());

    let tag = tag.into();
    let contents = fs::read_to_string(filename)
        .map(|v| v.tagged(&tag))
        .map_err(|err| {
            ShellError::labeled_error(
                &format!("Couldn't read config file:\n{}", err),
                "file name",
                &tag,
            )
        })?;

    let parsed: toml::Value = toml::from_str(&contents).map_err(|err| {
        ShellError::labeled_error(
            &format!("Couldn't parse config file:\n{}", err),
            "file name",
            &tag,
        )
    })?;

    let value = convert_toml_value_to_nu_value(&parsed, tag);
    let tag = value.tag();
    match value.value {
        UntaggedValue::Row(Dictionary { entries }) => Ok(entries),
        other => Err(ShellError::type_error(
            "Dictionary",
            other.type_name().spanned(tag.span),
        )),
    }
}

pub fn config(tag: impl Into<Tag>) -> Result<IndexMap<String, Value>, ShellError> {
    read(tag, &None)
}

pub fn write(config: &IndexMap<String, Value>, at: &Option<PathBuf>) -> Result<(), ShellError> {
    let filename = &mut default_path()?;
    let filename = match at {
        None => filename,
        Some(file) => {
            filename.pop();
            filename.push(file);
            filename
        }
    };

    let contents = value_to_toml_value(
        &UntaggedValue::Row(Dictionary::new(config.clone())).into_untagged_value(),
    )?;

    let contents = toml::to_string(&contents)?;

    fs::write(&filename, &contents)?;

    Ok(())
}

// A simple implementation of `% touch path` (ignores existing files)
fn touch(path: &Path) -> io::Result<()> {
    match OpenOptions::new().create(true).write(true).open(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
