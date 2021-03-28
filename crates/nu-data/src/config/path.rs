use crate::config::NuConfig;
use std::path::PathBuf;

pub const DEFAULT_CONFIG_LOCATION: &str = "config.toml";
const DEFAULT_HISTORY_LOCATION: &str = "history.txt";

pub fn history(config: &NuConfig) -> PathBuf {
    let default_path = crate::config::user_data()
        .map(|mut p| {
            p.push(DEFAULT_HISTORY_LOCATION);
            p
        })
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_HISTORY_LOCATION));

    let path = &config.var("history-path");

    path.as_ref().map_or(default_path.clone(), |custom_path| {
        match custom_path.as_string() {
            Ok(path) => PathBuf::from(path),
            Err(_) => default_path,
        }
    })
}

pub fn source_file(config: &NuConfig) -> PathBuf {
    match &config.source_file {
        Some(path) => PathBuf::from(path),
        None => {
            crate::config::default_path().unwrap_or_else(|_| PathBuf::from(DEFAULT_CONFIG_LOCATION))
        }
    }
}
