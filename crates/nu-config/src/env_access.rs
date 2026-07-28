//! Process environment and platform directory seam for path resolution.
//!
//! Production code uses [`SystemEnv`]. Unit tests inject values with
//! [`TestEnv`] so resolution can be verified without mutating the host
//! process environment.

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

/// Abstraction over process environment and platform directory discovery so
/// path-resolution logic can be unit-tested without touching the host.
///
/// # Seam
///
/// [`SystemEnv`] is the production adapter. [`TestEnv`] injects env vars and
/// platform directory fallbacks for tests.
///
/// Prefer [`Self::var_os`] over UTF-8-only access when paths may contain
/// non-Unicode bytes (Unix).
pub trait EnvAccess {
    /// Look up an environment variable as an [`OsString`] (preserves non-UTF-8).
    fn var_os(&self, name: &str) -> Option<OsString>;

    /// Platform config directory (e.g. `~/.config`), before the `nushell` suffix.
    fn config_dir(&self) -> Option<PathBuf>;

    /// Platform data directory (e.g. `~/.local/share`).
    fn data_dir(&self) -> Option<PathBuf>;

    /// Platform cache directory (e.g. `~/.cache`).
    fn cache_dir(&self) -> Option<PathBuf>;

    /// User home directory.
    fn home_dir(&self) -> Option<PathBuf>;

    /// Windows ProgramData folder. Unused on other OSes.
    #[cfg(windows)]
    fn program_data_dir(&self) -> Option<PathBuf>;

    /// UTF-8 convenience wrapper around [`Self::var_os`].
    fn var(&self, name: &str) -> Option<String> {
        self.var_os(name).and_then(|value| value.into_string().ok())
    }
}

/// Reads from the real process environment and `dirs` crate fallbacks.
pub struct SystemEnv;

impl EnvAccess for SystemEnv {
    fn var_os(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name)
    }

    fn config_dir(&self) -> Option<PathBuf> {
        dirs::config_dir()
    }

    fn data_dir(&self) -> Option<PathBuf> {
        dirs::data_dir()
    }

    fn cache_dir(&self) -> Option<PathBuf> {
        dirs::cache_dir()
    }

    fn home_dir(&self) -> Option<PathBuf> {
        dirs::home_dir()
    }

    #[cfg(windows)]
    fn program_data_dir(&self) -> Option<PathBuf> {
        // Prefer the known folder when available; fall back to the env var.
        // Call SHGetKnownFolderPath via our workspace `windows-sys` so we do not
        // pass GUID types across different `windows-sys` versions that `dirs-sys`
        // may resolve (those types are not interchangeable).
        known_folder_program_data().or_else(|| std::env::var_os("ProgramData").map(PathBuf::from))
    }
}

/// Resolve the Windows ProgramData known folder using the workspace `windows-sys`.
#[cfg(windows)]
fn known_folder_program_data() -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::slice;
    use windows_sys::Win32::Globalization::lstrlenW;
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{FOLDERID_ProgramData, SHGetKnownFolderPath};
    use windows_sys::core::PWSTR;

    // SAFETY: SHGetKnownFolderPath either returns a valid CoTaskMem-allocated
    // wide string (result == 0) or nothing we may read; we free the pointer in
    // both success and failure paths as the API requires.
    unsafe {
        let mut path_ptr: PWSTR = std::ptr::null_mut();
        let result = SHGetKnownFolderPath(
            &FOLDERID_ProgramData,
            0,
            std::ptr::null_mut(),
            &mut path_ptr,
        );
        if result == 0 && !path_ptr.is_null() {
            let len = lstrlenW(path_ptr) as usize;
            let path = slice::from_raw_parts(path_ptr, len);
            let ostr = OsString::from_wide(path);
            CoTaskMemFree(path_ptr.cast());
            Some(PathBuf::from(ostr))
        } else {
            if !path_ptr.is_null() {
                CoTaskMemFree(path_ptr.cast());
            }
            None
        }
    }
}

/// In-memory environment + optional platform directory overrides for tests.
///
/// Build with [`TestEnv::new`] or [`TestEnv::with_os_vars`], then chain
/// `with_*_dir` helpers for platform fallbacks.
#[derive(derive_setters::Setters)]
#[setters(prefix = "with_", strip_option, into)]
pub struct TestEnv {
    #[setters(skip)]
    vars: HashMap<String, OsString>,
    config_dir: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    #[cfg(windows)]
    program_data_dir: Option<PathBuf>,
}

impl TestEnv {
    /// Create a test env from UTF-8 key/value pairs.
    pub fn new(vars: HashMap<String, String>) -> Self {
        Self {
            vars: vars
                .into_iter()
                .map(|(k, v)| (k, OsString::from(v)))
                .collect(),
            config_dir: None,
            data_dir: None,
            cache_dir: None,
            home_dir: None,
            #[cfg(windows)]
            program_data_dir: None,
        }
    }

    /// Create a test env from raw [`OsString`] values (for non-UTF-8 path tests).
    pub fn with_os_vars(vars: HashMap<String, OsString>) -> Self {
        Self {
            vars,
            config_dir: None,
            data_dir: None,
            cache_dir: None,
            home_dir: None,
            #[cfg(windows)]
            program_data_dir: None,
        }
    }

    /// Insert or replace an env var as an [`OsStr`].
    pub fn insert_os(&mut self, name: impl Into<String>, value: impl AsRef<OsStr>) {
        self.vars.insert(name.into(), value.as_ref().to_os_string());
    }
}

impl EnvAccess for TestEnv {
    fn var_os(&self, name: &str) -> Option<OsString> {
        self.vars.get(name).cloned()
    }

    fn config_dir(&self) -> Option<PathBuf> {
        self.config_dir.clone()
    }

    fn data_dir(&self) -> Option<PathBuf> {
        self.data_dir.clone()
    }

    fn cache_dir(&self) -> Option<PathBuf> {
        self.cache_dir.clone()
    }

    fn home_dir(&self) -> Option<PathBuf> {
        self.home_dir.clone()
    }

    #[cfg(windows)]
    fn program_data_dir(&self) -> Option<PathBuf> {
        self.program_data_dir
            .clone()
            .or_else(|| self.var_os("ProgramData").map(PathBuf::from))
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::path::{Component, Path};

    /// Normalize for path comparisons that may differ only by separators or case.
    fn path_key(path: &Path) -> String {
        path.components()
            .map(|c| match c {
                Component::Prefix(p) => p.as_os_str().to_string_lossy().to_ascii_lowercase(),
                Component::RootDir => String::new(),
                Component::Normal(s) => s.to_string_lossy().to_ascii_lowercase(),
                Component::CurDir => ".".into(),
                Component::ParentDir => "..".into(),
            })
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("/")
    }

    #[test]
    fn known_folder_program_data_returns_absolute_existing_directory() {
        let path = known_folder_program_data().expect(
            "SHGetKnownFolderPath(FOLDERID_ProgramData) should succeed on Windows CI/hosts",
        );

        assert!(
            path.is_absolute(),
            "ProgramData known folder must be absolute, got {path:?}"
        );
        assert!(
            path.is_dir(),
            "ProgramData known folder must exist as a directory, got {path:?}"
        );
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some("ProgramData"),
            "expected final component to be ProgramData, got {path:?}"
        );
    }

    #[test]
    fn system_env_program_data_dir_uses_known_folder_when_available() {
        let from_api = known_folder_program_data()
            .expect("known folder lookup should succeed on Windows CI/hosts");
        let from_system = SystemEnv
            .program_data_dir()
            .expect("SystemEnv::program_data_dir should resolve ProgramData");

        assert_eq!(
            path_key(&from_system),
            path_key(&from_api),
            "SystemEnv should prefer the known-folder path over env fallback; \
             system={from_system:?} api={from_api:?}"
        );

        // When the common env var is set, it should agree with the known folder
        // (case/separator differences aside).
        if let Some(from_env) = std::env::var_os("ProgramData").map(PathBuf::from) {
            assert_eq!(
                path_key(&from_api),
                path_key(&from_env),
                "known folder and %ProgramData% should resolve to the same location; \
                 api={from_api:?} env={from_env:?}"
            );
        }
    }
}
