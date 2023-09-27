use etcetera::BaseStrategy;
#[cfg(windows)]
use omnipath::WinPathExt;
use once_cell::sync::Lazy;
use std::path::PathBuf;

static HOME_DIR: Lazy<Option<PathBuf>> = Lazy::new(|| etcetera::home_dir().ok());
static CONFIG_DIR: Lazy<Option<PathBuf>> = Lazy::new(|| {
    etcetera::choose_base_strategy()
        .map(|s| s.config_dir())
        .ok()
});

/// Returns the home directory of the current user.
///
/// Uses the `HOME` environment variable on Linux and macOS, if set, otherwise `getpwuid_r`.
/// Uses the `USERPROFILE` environment variable on Windows, if set, otherwise `SHGetKnownFolderPath` with `CSIDL_PROFILE`.
pub fn home_dir() -> Option<PathBuf> {
    HOME_DIR.clone()
}

/// Returns the path where to the nushell config directory.
///
/// Looks for the following based on the OS:
///   - Linux: `${XDG_CONFIG_HOME}/nushell`
///   - macOS: `${XDG_CONFIG_HOME}/nushell`
///   - Windows: `~\AppData\Roaming\nushell`
pub fn config_dir() -> Option<PathBuf> {
    CONFIG_DIR.clone()
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
