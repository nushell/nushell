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
            return Some(config_dir);
        }
        Some(config_dir)
    } else {
        None
    };

    if let Ok(basedirs) = etcetera::base_strategy::choose_native_strategy() {
        let config_home = if cfg!(target_os = "macos") {
            basedirs.data_dir()
        } else {
            basedirs.config_dir()
        };
        if config_home.join("nushell").is_dir() {
            return Some(config_home);
        }
    }

    default
});

pub fn home_dir() -> Option<PathBuf> {
    HOME_DIR.clone()
}

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
