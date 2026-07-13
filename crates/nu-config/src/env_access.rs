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
        dirs_sys::known_folder(windows_sys::Win32::UI::Shell::FOLDERID_ProgramData)
            .or_else(|| std::env::var_os("ProgramData").map(PathBuf::from))
    }
}

/// In-memory environment + optional platform directory overrides for tests.
///
/// Build with [`TestEnv::new`] or [`TestEnv::with_os_vars`], then chain
/// `with_*_dir` helpers for platform fallbacks.
pub struct TestEnv {
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

    pub fn with_config_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.config_dir = Some(path.into());
        self
    }

    pub fn with_data_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.data_dir = Some(path.into());
        self
    }

    pub fn with_cache_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.cache_dir = Some(path.into());
        self
    }

    pub fn with_home_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.home_dir = Some(path.into());
        self
    }

    #[cfg(windows)]
    pub fn with_program_data_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.program_data_dir = Some(path.into());
        self
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
