use std::path::PathBuf;

use crate::AbsolutePathBuf;

pub fn home_dir() -> Option<AbsolutePathBuf> {
    dirs::home_dir().and_then(|home| AbsolutePathBuf::try_from(home).ok())
}

/// Return the data directory for the current platform or XDG_DATA_HOME if specified.
pub fn data_dir() -> Option<AbsolutePathBuf> {
    configurable_dir_path("XDG_DATA_HOME", dirs::data_dir)
}

/// Return the cache directory for the current platform or XDG_CACHE_HOME if specified.
pub fn cache_dir() -> Option<AbsolutePathBuf> {
    configurable_dir_path("XDG_CACHE_HOME", dirs::cache_dir)
}

/// Return the nushell config directory.
pub fn nu_config_dir() -> Option<AbsolutePathBuf> {
    configurable_dir_path("XDG_CONFIG_HOME", dirs::config_dir).map(|mut p| {
        p.push("nushell");
        p
    })
}

fn configurable_dir_path(
    name: &'static str,
    dir: impl FnOnce() -> Option<PathBuf>,
) -> Option<AbsolutePathBuf> {
    std::env::var(name)
        .ok()
        .and_then(|path| AbsolutePathBuf::try_from(path).ok())
        .or_else(|| dir().and_then(|path| AbsolutePathBuf::try_from(path).ok()))
        .map(|path| path.canonicalize().map(Into::into).unwrap_or(path))
}
