//! Environment variable name handling with platform-specific case sensitivity.
//!
//! This module defines `EnvName`, a wrapper around `String` that handles case sensitivity
//! for environment variable names according to platform conventions and Nushell's needs.
//!
//! ## Case Sensitivity Rules
//!
//! - **Windows**: All environment variable names are case-insensitive but case-preserving.
//!   This matches Windows' behavior where `PATH`, `Path`, and `path` refer to the same variable.
//!
//! - **Other platforms (macOS, Linux, etc.)**: Environment variable names are case-sensitive,
//!   meaning `HOME` and `home` are different variables.
//!
//! - **Special case for PATH**: Regardless of platform, the `PATH` environment variable
//!   (and its variants like `Path`, `path`, `pAtH`) is treated as case-insensitive on all platforms.
//!   This ensures compatibility with existing scripts and external command execution.
//!
//! This ensures that:
//! - `$env.PATH`, `$env.path`, `$env.Path` all work on any platform
//! - Other env vars like `$env.HOME` vs `$env.home` remain distinct on Unix
//! - Windows continues to work as before
//! - Existing tests and scripts don't break

use std::fmt;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

/// A `String` that's case-insensitive for PATH on all platforms, and case-insensitive on Windows but case-sensitive on other platforms for other env vars, used for environment variable names.
#[derive(Clone)]
pub struct EnvName(pub(crate) String);

impl Debug for EnvName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: Into<String>> From<T> for EnvName {
    fn from(name: T) -> Self {
        EnvName(name.into())
    }
}

impl AsRef<str> for EnvName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<Self> for EnvName {
    fn eq(&self, other: &Self) -> bool {
        // Special handling for PATH: case-insensitive on all platforms
        if self.is_path_env_var() || other.is_path_env_var() {
            self.0.eq_ignore_ascii_case(&other.0)
        } else {
            // Platform-specific case sensitivity for other variables
            #[cfg(windows)]
            {
                self.0.eq_ignore_ascii_case(&other.0)
            }

            #[cfg(not(windows))]
            {
                self.0 == other.0
            }
        }
    }
}

impl Eq for EnvName {}

impl Hash for EnvName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Special handling for PATH: case-insensitive hashing on all platforms
        if self.is_path_env_var() {
            self.hash_case_insensitive(state);
        } else {
            // Platform-specific hashing for other variables
            #[cfg(windows)]
            {
                self.hash_case_insensitive(state);
            }

            #[cfg(not(windows))]
            {
                // Hash case-sensitively
                self.0.hash(state);
            }
        }
    }
}

impl EnvName {
    /// Get the inner string
    pub fn into_string(self) -> String {
        self.0
    }

    /// Get a reference to the inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if this environment variable name is "PATH" (case-insensitive)
    fn is_path_env_var(&self) -> bool {
        self.0.eq_ignore_ascii_case("path")
    }

    /// Hash the name case-insensitively by uppercasing each byte
    fn hash_case_insensitive<H: Hasher>(&self, state: &mut H) {
        for &b in self.0.as_bytes() {
            b.to_ascii_uppercase().hash(state);
        }
    }
}

#[cfg(windows)]
#[test]
fn test_env_name_windows() {
    // On Windows, all environment variables are case-insensitive
    let strings1 = [
        EnvName::from("abc"),
        EnvName::from("Abc"),
        EnvName::from("aBc"),
        EnvName::from("abC"),
        EnvName::from("ABc"),
        EnvName::from("aBC"),
        EnvName::from("AbC"),
        EnvName::from("ABC"),
    ];
    let strings2 = [
        EnvName::from("xyz"),
        EnvName::from("Xyz"),
        EnvName::from("xYz"),
        EnvName::from("xyZ"),
        EnvName::from("XYz"),
        EnvName::from("xYZ"),
        EnvName::from("XyZ"),
        EnvName::from("XYZ"),
    ];
    // All the strings in `strings1` compare equal to each other and hash the same on Windows.
    for s1 in &strings1 {
        for also_s1 in &strings1 {
            assert_eq!(s1, also_s1);
            let mut hash_set = std::collections::HashSet::new();
            hash_set.insert(s1);
            hash_set.insert(also_s1);
            assert_eq!(hash_set.len(), 1);
        }
    }
    // Same for `strings2`.
    for s2 in &strings2 {
        for also_s2 in &strings2 {
            assert_eq!(s2, also_s2);
            let mut hash_set = std::collections::HashSet::new();
            hash_set.insert(s2);
            hash_set.insert(also_s2);
            assert_eq!(hash_set.len(), 1);
        }
    }
    // But nothing in `strings1` compares equal to anything in `strings2`. We also assert that
    // their hashes are distinct. In theory they could collide, but using DefaultHasher here (which
    // is initialized with the zero key) should prevent that from happening randomly.
    for s1 in &strings1 {
        for s2 in &strings2 {
            assert_ne!(s1, s2);
            let mut hasher1 = std::hash::DefaultHasher::new();
            s1.hash(&mut hasher1);
            let mut hasher2 = std::hash::DefaultHasher::new();
            s2.hash(&mut hasher2);
            assert_ne!(hasher1.finish(), hasher2.finish());
        }
    }
}

#[cfg(not(windows))]
#[test]
fn test_env_name_unix() {
    // On Unix-like systems, most environment variables are case-sensitive
    let strings = [
        EnvName::from("Abc"),
        EnvName::from("aBc"),
        EnvName::from("abC"),
        EnvName::from("ABc"),
        EnvName::from("aBC"),
        EnvName::from("AbC"),
        EnvName::from("ABC"),
    ];
    // None of these strings compare equal to "abc" on Unix. We also assert that their hashes are
    // distinct. In theory they could collide, but using DefaultHasher here (which is
    // initialized with the zero key) should prevent that from happening randomly.
    for s in &strings {
        assert_ne!(&EnvName::from("abc"), s);
        let mut hasher1 = std::hash::DefaultHasher::new();
        EnvName::from("abc").hash(&mut hasher1);
        let mut hasher2 = std::hash::DefaultHasher::new();
        s.hash(&mut hasher2);
        assert_ne!(hasher1.finish(), hasher2.finish());
    }

    // But PATH variants should be equal and hash the same on all platforms
    let path_strings = [
        EnvName::from("path"),
        EnvName::from("Path"),
        EnvName::from("PATH"),
        EnvName::from("pAtH"),
    ];
    for s in &path_strings {
        assert_eq!(&EnvName::from("PATH"), s);
        let mut hasher1 = std::hash::DefaultHasher::new();
        EnvName::from("PATH").hash(&mut hasher1);
        let mut hasher2 = std::hash::DefaultHasher::new();
        s.hash(&mut hasher2);
        assert_eq!(hasher1.finish(), hasher2.finish());
    }
}
