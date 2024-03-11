#[cfg(windows)]
use omnipath::WinPathExt;
use std::path::PathBuf;

pub fn home_dir() -> Option<PathBuf> {
    dirs_next::home_dir()
}

pub fn config_dir() -> Option<PathBuf> {
    match std::env::var("XDG_CONFIG_HOME").map(PathBuf::from) {
        Ok(xdg_config) if xdg_config.is_absolute() => {
            Some(canonicalize(&xdg_config).unwrap_or(xdg_config))
        }
        _ => config_dir_old(),
    }
}

/// Get the old default config directory. Outside of Linux, this will ignore `XDG_CONFIG_HOME`
pub fn config_dir_old() -> Option<PathBuf> {
    let path = dirs_next::config_dir()?;
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
