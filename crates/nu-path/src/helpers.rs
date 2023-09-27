use etcetera::BaseStrategy;
#[cfg(windows)]
use omnipath::WinPathExt;
use once_cell::sync::Lazy;
use std::path::PathBuf;

static HOME_DIR: Lazy<Option<PathBuf>> = Lazy::new(|| etcetera::home_dir().ok());
static CONFIG_DIR: Lazy<Option<PathBuf>> = Lazy::new(|| {
    etcetera::base_strategy::choose_native_strategy()
        .ok()
        .and_then(|strategy| {
            // For historical reasons, nushell prefers to use ~/Library/Application Support/
            // rather than ~/Library/Preferences/. See https://github.com/nushell/nushell/pull/8682.
            let dir = if cfg!(target_os = "macos") {
                strategy.data_dir()
            } else {
                strategy.config_dir()
            };

            if dir.join("nushell").is_dir() {
                Some(dir)
            } else {
                home_dir().map(|dir| dir.join(".config"))
            }
        })
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
/// Looks for the following directories based on the OS. If the directory exists,
/// the parent config directory is returned; otherwise, `~/.config` is returned.
///   - Linux: `${XDG_CONFIG_HOME}/nushell`
///   - macOS: `~/Library/Application Support/nushell`
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
