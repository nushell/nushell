use crate::AbsolutePathBuf;

pub fn home_dir() -> Option<AbsolutePathBuf> {
    dirs_next::home_dir().and_then(|home| AbsolutePathBuf::try_from(home).ok())
}

/// Return the data directory for the current platform or XDG_DATA_HOME if specified.
pub fn data_dir() -> Option<AbsolutePathBuf> {
    std::env::var("XDG_DATA_HOME")
        .ok()
        .and_then(|path| AbsolutePathBuf::try_from(path).ok())
        .or_else(|| dirs_next::data_dir().and_then(|path| AbsolutePathBuf::try_from(path).ok()))
        .map(|path| path.canonicalize().map(Into::into).unwrap_or(path))
}

/// Return the cache directory for the current platform or XDG_CACHE_HOME if specified.
pub fn cache_dir() -> Option<AbsolutePathBuf> {
    std::env::var("XDG_CACHE_HOME")
        .ok()
        .and_then(|path| AbsolutePathBuf::try_from(path).ok())
        .or_else(|| dirs_next::cache_dir().and_then(|path| AbsolutePathBuf::try_from(path).ok()))
        .map(|path| path.canonicalize().map(Into::into).unwrap_or(path))
}

/// Return the config directory for the current platform or XDG_CONFIG_HOME if specified.
pub fn config_dir() -> Option<AbsolutePathBuf> {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .and_then(|path| AbsolutePathBuf::try_from(path).ok())
        .or_else(|| dirs_next::config_dir().and_then(|path| AbsolutePathBuf::try_from(path).ok()))
        .map(|path| path.canonicalize().map(Into::into).unwrap_or(path))
}
