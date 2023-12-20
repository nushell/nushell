#[cfg(windows)]
use omnipath::WinPathExt;
use std::path::PathBuf;

pub fn home_dir() -> Option<PathBuf> {
    dirs_next::home_dir()
}

pub fn config_dir() -> Option<PathBuf> {
    dirs_next::config_dir()
}

pub fn vendor_completions_dirs() -> Vec<PathBuf> {
    let vendor_completions_fn = |mut path: PathBuf| {
        path.push("nushell");
        path.push("completions");
        path
    };
    let mut dirs = Vec::new();

    // default global directory
    #[cfg(not(target_os = "windows"))]
    let global_default = Some(PathBuf::from("/usr/share"));
    #[cfg(target_os = "windows")]
    let global_default =
        dirs_sys_next::known_folder(&winapi::um::knownfolders::FOLDERID_ProgramData);

    // global directory
    if let Some(global) = std::env::var("NU_VENDOR_COMPLETIONS_DIR")
        .ok()
        .or(option_env!("NU_VENDOR_COMPLETIONS_DIR").map(String::from))
        .map(PathBuf::from)
        .or(global_default.map(vendor_completions_fn))
    {
        dirs.push(global);
    }

    // local directory of the current user
    if let Some(data_dir) = std::env::var("NU_COMPLETIONS_DIR")
        .ok()
        .map(PathBuf::from)
        .or_else(|| dirs_next::data_dir().map(vendor_completions_fn))
    {
        dirs.push(data_dir);
    }

    dirs
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
