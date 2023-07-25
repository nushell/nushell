use etcetera::BaseStrategy;
#[cfg(windows)]
use omnipath::WinPathExt;
use once_cell::sync::Lazy;
use std::path::PathBuf;

static HOME_DIR: Lazy<Option<PathBuf>> = Lazy::new(|| etcetera::home_dir().ok());
static CONFIG_DIR: Lazy<Option<PathBuf>> = Lazy::new(|| {
    let default = if let Some(mut config_dir) = home_dir() {
        config_dir.push(".config");
        if config_dir.join("nushell").is_dir() {
            // `~/.config/nushell` exists, so we'll use that
            return Some(config_dir);
        }
        // `~/.config/nushell` doesn't exist, but we'll use it if the "native" folder doesn't exist either
        Some(config_dir)
    } else {
        // `~` not found, so we'll return `None` if the "native" folder doesn't exist either
        None
    };

    if let Ok(basedirs) = etcetera::base_strategy::choose_native_strategy() {
        let config_home = if cfg!(target_os = "macos") {
            basedirs.data_dir()
        } else {
            basedirs.config_dir()
        };
        if config_home.join("nushell").is_dir() {
            // "native" config folder exists, so we'll use that
            return Some(config_home);
        }
    }

    // fresh install, so we'll use the default defined above
    default
});

/// Returns the home directory of the current user.
///
/// Uses the `HOME` environment variable on Linux and macOS, if set, otherwise `getpwuid_r`.
/// Uses the `USERPROFILE` environment variable on Windows, if set, otherwise `SHGetKnownFolderPath` with `CSIDL_PROFILE`.
pub fn home_dir() -> Option<PathBuf> {
    HOME_DIR.clone()
}

/// Returns the path where the `nushell` folder should be located.
///
/// If `~/.config/nushell` exists, returns `~/.config`.
/// Checks the following path based on the OS:
///   - Linux: `~/.config/`
///   - macOS: `~/Library/Application Support/`
///   - Windows: `{FOLDERID_RoamingAppData}`
/// If the `nushell` folder exists in the path, returns the path.
/// Otherwise, returns `~/.config'.
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
