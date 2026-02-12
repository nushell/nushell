use std::fmt;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

/// A `String` that's case-insensitive on Windows but case-sensitive on other platforms, used for environment variable names.
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
        // On Windows, env var names are case-insensitive
        #[cfg(windows)]
        {
            self.0.eq_ignore_ascii_case(&other.0)
        }

        // On all other platforms, env var names are case-sensitive.
        #[cfg(not(windows))]
        {
            self.0 == other.0
        }
    }
}

impl Eq for EnvName {}

impl Hash for EnvName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // On Windows, env var names are case-insensitive
        #[cfg(windows)]
        {
            // Uppercase each byte separately to avoid reallocating. This is similar to what
            // eq_ignore_ascii_case does internally.
            for &b in self.0.as_bytes() {
                b.to_ascii_uppercase().hash(state);
            }
        }

        // On all other platforms, env var names are case-sensitive.
        #[cfg(not(windows))]
        {
            self.0.hash(state);
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
}

#[cfg(windows)]
#[test]
fn test_env_name_windows() {
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
}
