#[cfg(windows)]
use omnipath::WinPathExt;
use std::path::PathBuf;

pub fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

/// Return the data directory for the current platform or XDG_DATA_HOME if specified.
pub fn data_dir() -> Option<PathBuf> {
    match std::env::var("XDG_DATA_HOME").map(PathBuf::from) {
        Ok(xdg_data) if xdg_data.is_absolute() => Some(canonicalize(&xdg_data).unwrap_or(xdg_data)),
        _ => get_canonicalized_path(dirs::data_dir()),
    }
}

/// Return the cache directory for the current platform or XDG_CACHE_HOME if specified.
pub fn cache_dir() -> Option<PathBuf> {
    match std::env::var("XDG_CACHE_HOME").map(PathBuf::from) {
        Ok(xdg_cache) if xdg_cache.is_absolute() => {
            Some(canonicalize(&xdg_cache).unwrap_or(xdg_cache))
        }
        _ => get_canonicalized_path(dirs::cache_dir()),
    }
}

/// Return the config directory for the current platform or XDG_CONFIG_HOME if specified.
pub fn config_dir() -> Option<PathBuf> {
    match std::env::var("XDG_CONFIG_HOME").map(PathBuf::from) {
        Ok(xdg_config) if xdg_config.is_absolute() => {
            Some(canonicalize(&xdg_config).unwrap_or(xdg_config))
        }
        _ => get_canonicalized_path(dirs::config_dir()),
    }
}

pub fn get_canonicalized_path(path: Option<PathBuf>) -> Option<PathBuf> {
    let path = path?;
    Some(canonicalize(&path).unwrap_or(path))
}

#[cfg(windows)]
pub fn canonicalize(path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    path.canonicalize()?.to_winuser_path()
}
#[cfg(not(windows))]
pub fn canonicalize(path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    path.canonicalize()
}

#[cfg(windows)]
pub fn simiplified(path: &std::path::Path) -> PathBuf {
    path.to_winuser_path()
        .unwrap_or_else(|_| path.to_path_buf())
}
#[cfg(not(windows))]
pub fn simiplified(path: &std::path::Path) -> PathBuf {
    path.to_path_buf()
}
