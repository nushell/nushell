use crate::commands::from_toml::convert_toml_value_to_nu_value;
use crate::commands::to_toml::value_to_toml_value;
use crate::data::{Dictionary, Value};
use crate::errors::ShellError;
use crate::prelude::*;
use app_dirs::*;
use indexmap::IndexMap;
use log::trace;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Serialize)]
struct Config {
    #[serde(flatten)]
    extra: IndexMap<String, Tagged<Value>>,
}

pub const APP_INFO: AppInfo = AppInfo {
    name: "nu",
    author: "nu shell developers",
};

pub fn config_path() -> Result<PathBuf, ShellError> {
    app_path(AppDataType::UserConfig, "config")
}

pub fn default_path() -> Result<PathBuf, ShellError> {
    default_path_for(&None)
}

pub fn default_path_for(file: &Option<PathBuf>) -> Result<PathBuf, ShellError> {
    let filename = &mut config_path()?;
    let filename = match file {
        None => {
            filename.push("config.toml");
            filename
        }
        Some(file) => {
            filename.push(file);
            filename
        }
    };

    Ok(filename.clone())
}

pub fn user_data() -> Result<PathBuf, ShellError> {
    app_path(AppDataType::UserData, "user data")
}

pub fn app_path(app_data_type: AppDataType, display: &str) -> Result<PathBuf, ShellError> {
    let path = app_root(app_data_type, &APP_INFO).map_err(|err| {
        ShellError::untagged_runtime_error(&format!("Couldn't open {} path:\n{}", display, err))
    })?;

    Ok(path)
}

pub fn read(
    tag: impl Into<Tag>,
    at: &Option<PathBuf>,
) -> Result<IndexMap<String, Tagged<Value>>, ShellError> {
    let filename = default_path()?;

    let filename = match at {
        None => filename,
        Some(ref file) => file.clone(),
    };

    touch(&filename)?;

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
    match value.item {
        Value::Row(Dictionary { entries }) => Ok(entries),
        other => Err(ShellError::type_error(
            "Dictionary",
            other.type_name().spanned(tag.span),
        )),
    }
}

pub(crate) fn config(tag: impl Into<Tag>) -> Result<IndexMap<String, Tagged<Value>>, ShellError> {
    read(tag, &None)
}

pub fn write(
    config: &IndexMap<String, Tagged<Value>>,
    at: &Option<PathBuf>,
) -> Result<(), ShellError> {
    let filename = &mut default_path()?;
    let filename = match at {
        None => filename,
        Some(file) => {
            filename.pop();
            filename.push(file);
            filename
        }
    };

    let contents =
        value_to_toml_value(&Value::Row(Dictionary::new(config.clone())).tagged_unknown())?;

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
