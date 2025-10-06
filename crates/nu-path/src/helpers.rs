#[cfg(windows)]
use std::path::{Component, Prefix};
use std::path::{Path, PathBuf};

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

// List of special paths that can be written to and/or read from, even though they
// don't appear as directory entries.
// See https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file
// In rare circumstances, reserved paths _can_ exist as regular files in a
// directory which shadow their special counterpart, so the safe way of referring
// to these paths is by prefixing them with '\\.\' (this instructs the Windows APIs
// to access the Win32 device namespace instead of the Win32 file namespace)
// https://learn.microsoft.com/en-us/dotnet/standard/io/file-path-formats
#[cfg(windows)]
pub fn is_windows_device_path(path: &Path) -> bool {
    match path.components().next() {
        Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::DeviceNS(_)) => {
            return true;
        }
        _ => {}
    }
    let special_paths: [&Path; 28] = [
        Path::new("CON"),
        Path::new("PRN"),
        Path::new("AUX"),
        Path::new("NUL"),
        Path::new("COM1"),
        Path::new("COM2"),
        Path::new("COM3"),
        Path::new("COM4"),
        Path::new("COM5"),
        Path::new("COM6"),
        Path::new("COM7"),
        Path::new("COM8"),
        Path::new("COM9"),
        Path::new("COM¹"),
        Path::new("COM²"),
        Path::new("COM³"),
        Path::new("LPT1"),
        Path::new("LPT2"),
        Path::new("LPT3"),
        Path::new("LPT4"),
        Path::new("LPT5"),
        Path::new("LPT6"),
        Path::new("LPT7"),
        Path::new("LPT8"),
        Path::new("LPT9"),
        Path::new("LPT¹"),
        Path::new("LPT²"),
        Path::new("LPT³"),
    ];
    if special_paths.contains(&path) {
        return true;
    }
    false
}

#[cfg(not(windows))]
pub fn is_windows_device_path(_path: &Path) -> bool {
    false
}

#[cfg(test)]
mod test_is_windows_device_path {
    use crate::is_windows_device_path;
    use std::path::Path;

    #[cfg_attr(not(windows), ignore = "only for Windows")]
    #[test]
    fn device_namespace() {
        assert!(is_windows_device_path(Path::new(r"\\.\CON")))
    }

    #[cfg_attr(not(windows), ignore = "only for Windows")]
    #[test]
    fn reserved_device_name() {
        assert!(is_windows_device_path(Path::new(r"NUL")))
    }

    #[cfg_attr(not(windows), ignore = "only for Windows")]
    #[test]
    fn normal_path() {
        assert!(!is_windows_device_path(Path::new(r"dir\file")))
    }

    #[cfg_attr(not(windows), ignore = "only for Windows")]
    #[test]
    fn absolute_path() {
        assert!(!is_windows_device_path(Path::new(r"\dir\file")))
    }

    #[cfg_attr(not(windows), ignore = "only for Windows")]
    #[test]
    fn unc_path() {
        assert!(!is_windows_device_path(Path::new(r"\\server\share")))
    }

    #[cfg_attr(not(windows), ignore = "only for Windows")]
    #[test]
    fn verbatim_path() {
        assert!(!is_windows_device_path(Path::new(r"\\?\dir\file")))
    }
}
