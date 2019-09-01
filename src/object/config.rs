use crate::commands::from_toml::convert_toml_value_to_nu_value;
use crate::commands::to_toml::value_to_toml_value;
use crate::errors::ShellError;
use crate::object::{Dictionary, Value};
use crate::prelude::*;
use app_dirs::*;
use indexmap::IndexMap;
use log::trace;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

const APP_INFO: AppInfo = AppInfo {
    name: "nu",
    author: "nu shell developers",
};

#[derive(Deserialize, Serialize)]
struct Config {
    #[serde(flatten)]
    extra: IndexMap<String, Tagged<Value>>,
}

pub(crate) fn config_path() -> Result<PathBuf, ShellError> {
    let location = app_root(AppDataType::UserConfig, &APP_INFO)
        .map_err(|err| ShellError::string(&format!("Couldn't open config file:\n{}", err)))?;

    Ok(location.join("config.toml"))
}

pub(crate) fn write_config(config: &IndexMap<String, Tagged<Value>>) -> Result<(), ShellError> {
    let location = app_root(AppDataType::UserConfig, &APP_INFO)
        .map_err(|err| ShellError::string(&format!("Couldn't open config file:\n{}", err)))?;

    let filename = location.join("config.toml");
    touch(&filename)?;

    let contents =
        value_to_toml_value(&Value::Object(Dictionary::new(config.clone())).tagged_unknown())?;

    let contents = toml::to_string(&contents)?;

    fs::write(&filename, &contents)?;

    Ok(())
}

pub(crate) fn config(span: impl Into<Span>) -> Result<IndexMap<String, Tagged<Value>>, ShellError> {
    let span = span.into();

    let location = app_root(AppDataType::UserConfig, &APP_INFO)
        .map_err(|err| ShellError::string(&format!("Couldn't open config file:\n{}", err)))?;

    let filename = location.join("config.toml");
    touch(&filename)?;

    trace!("config file = {}", filename.display());

    let contents = fs::read_to_string(filename)
        .map(|v| v.simple_spanned(span))
        .map_err(|err| ShellError::string(&format!("Couldn't read config file:\n{}", err)))?;

    let parsed: toml::Value = toml::from_str(&contents)
        .map_err(|err| ShellError::string(&format!("Couldn't parse config file:\n{}", err)))?;

    let value = convert_toml_value_to_nu_value(&parsed, Tag::unknown_origin(span));
    let tag = value.tag();
    match value.item {
        Value::Object(Dictionary { entries }) => Ok(entries),
        other => Err(ShellError::type_error(
            "Dictionary",
            other.type_name().tagged(tag),
        )),
    }
}

// A simple implementation of `% touch path` (ignores existing files)
fn touch(path: &Path) -> io::Result<()> {
    match OpenOptions::new().create(true).write(true).open(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
