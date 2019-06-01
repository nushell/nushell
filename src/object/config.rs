use crate::errors::ShellError;
use crate::prelude::*;
use app_dirs::*;
use indexmap::IndexMap;
use log::trace;
use serde_derive::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io;
use std::path::Path;

const APP_INFO: AppInfo = AppInfo {
    name: "nu",
    author: "nu shell developers",
};

#[derive(Deserialize, Serialize)]
struct Config {
    #[serde(flatten)]
    extra: IndexMap<String, Value>,
}

crate fn write_config(map: &IndexMap<String, Value>) -> Result<(), ShellError> {
    let location = app_root(AppDataType::UserConfig, &APP_INFO)
        .map_err(|err| ShellError::string(&format!("Couldn't open config file:\n{}", err)))?;

    let filename = location.join("config.toml");
    touch(&filename)?;

    let contents = toml::to_string(&Config { extra: map.clone() })?;

    fs::write(&filename, &contents)?;

    Ok(())
}

crate fn config() -> Result<IndexMap<String, Value>, ShellError> {
    let location = app_root(AppDataType::UserConfig, &APP_INFO)
        .map_err(|err| ShellError::string(&format!("Couldn't open config file:\n{}", err)))?;

    let filename = location.join("config.toml");
    touch(&filename)?;

    trace!("config file = {}", filename.display());

    let contents = fs::read_to_string(filename)
        .map_err(|err| ShellError::string(&format!("Couldn't read config file:\n{}", err)))?;

    let parsed: Config = toml::from_str(&contents)
        .map_err(|err| ShellError::string(&format!("Couldn't parse config file:\n{}", err)))?;

    Ok(parsed.extra)
}

// A simple implementation of `% touch path` (ignores existing files)
fn touch(path: &Path) -> io::Result<()> {
    match OpenOptions::new().create(true).write(true).open(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
