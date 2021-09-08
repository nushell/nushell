use std::path::PathBuf;

use super::NuConfig;

const DEFAULT_LOCATION: &str = "history.txt";

pub fn default_history_path() -> PathBuf {
    crate::config::user_data()
        .map(|mut p| {
            p.push(DEFAULT_LOCATION);
            p
        })
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_LOCATION))
}

/// Get history path of config, if present
pub fn history_path(config: &NuConfig) -> Option<PathBuf> {
    config
        .var("history-path")
        .and_then(|custom_path| custom_path.as_string().map(PathBuf::from).ok())
}

/// Get history path in config or default
pub fn history_path_or_default(config: &NuConfig) -> PathBuf {
    history_path(config).unwrap_or_else(default_history_path)
}
