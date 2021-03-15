use nu_data::config::Conf;
use std::path::PathBuf;

const DEFAULT_LOCATION: &str = "history.txt";

pub fn default_history_path() -> PathBuf {
    nu_data::config::user_data()
        .map(|mut p| {
            p.push(DEFAULT_LOCATION);
            p
        })
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_LOCATION))
}

pub fn history_path(config: &dyn Conf) -> PathBuf {
    let default_history_path = default_history_path();

    config.var("history-path").map_or(
        default_history_path.clone(),
        |custom_path| match custom_path.as_string() {
            Ok(path) => PathBuf::from(path),
            Err(_) => default_history_path,
        },
    )
}
