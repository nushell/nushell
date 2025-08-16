use std::{
    borrow::Borrow,
    fmt::{Arguments, Debug, Display},
    hash::Hash,
    ops::Deref,
};

use serde::{Deserialize, Serialize};

/// An owned, immutable string with compact storage and efficient cloning.
///
/// `SharedString` is designed for immutable strings that are frequently cloned and hold ownership.
/// It offers similar characteristics to [`Arc<str>`](std::sync::Arc) but with several key
/// optimizations for improved efficiency and memory usage:
///
/// - **Efficient Cloning:**
///   Cloning a `SharedString` is very inexpensive.
///   It typically involves just a pointer copy and an atomic reference count increment, without
///   copying the actual string data.
///
/// - **Small String Optimization (SSO):**
///   For shorter strings, the data is stored directly within the `SharedString` struct, keeping
///   the data on the stack and avoiding indirection.
///
/// - **Static String Re-use:**
///   Strings with a `'static` lifetime are directly referenced, avoiding unnecessary copies or
///   allocations.
///
/// - **Niche Optimization:**
///   `SharedString` allows niche-optimization, meaning that [`Option<SharedString>`] has the same
///   memory footprint as `SharedString`.
///
/// - **Compact Size:**
///   On 64-bit systems, `SharedString` is 16 bytes.
///   This is achieved by disregarding the capacity of a `String` since we only hold the string as
///   immutable.
///   
/// Internally, `SharedString` is powered by [`lean_string::LeanString`], which provides the
/// underlying implementation for these optimizations.
pub struct SharedString(lean_string::LeanString);

const _: () = const {
    assert!(size_of::<SharedString>() == size_of::<[usize; 2]>());
    assert!(size_of::<SharedString>() == size_of::<Option<SharedString>>());
};

impl SharedString {
    /// Returns a string slice containing the entire `SharedString`.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Returns a byte slice of this `SharedString`'s contents.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Returns the length of this `SharedString`, in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the `SharedString` has a length of 0, `false` otherwise.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns a `SharedString` by taking ownership of an allocation.
    #[inline]
    pub fn from_string(string: String) -> Self {
        Self(lean_string::LeanString::from(string))
    }

    /// Returns a `SharedString` pointing to the given slice, without copying.
    ///
    /// By using this function instead of [`from_string`](Self::from_string), we can avoid any
    /// copying and always refer to the provided static string slice.
    #[inline]
    pub fn from_static_str(string: &'static str) -> Self {
        Self(lean_string::LeanString::from_static_str(string))
    }

    /// Builds a new `SharedString` from the given formatting arguments.
    ///
    /// You can get an [`Arguments`] instance by calling [`format_args!`].
    /// This function is used when using [`sformat!`](crate::sformat) instead of [`format!`] to
    /// create a `SharedString`.
    #[inline]
    pub fn from_fmt(arguments: Arguments) -> Self {
        match arguments.as_str() {
            Some(static_str) => Self::from_static_str(static_str),
            None => Self::from_string(std::fmt::format(arguments)),
        }
    }
}

impl AsRef<str> for SharedString {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for SharedString {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Clone for SharedString {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Debug for SharedString {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Default for SharedString {
    #[inline]
    fn default() -> Self {
        Self(Default::default())
    }
}

impl Deref for SharedString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl Display for SharedString {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Eq for SharedString {}

impl From<String> for SharedString {
    #[inline]
    fn from(string: String) -> Self {
        Self::from_string(string)
    }
}

impl From<&'static str> for SharedString {
    #[inline]
    fn from(string: &'static str) -> Self {
        Self::from_static_str(string)
    }
}

impl Hash for SharedString {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Ord for SharedString {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialEq for SharedString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd for SharedString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Serialize for SharedString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SharedString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self(lean_string::LeanString::deserialize(deserializer)?))
    }
}

/// A macro for creating [`SharedString`] instances from format arguments.
///
/// This macro works similarly to [`format!`] but returns a [`SharedString`] instead of a [`String`].
/// It attempts to optimize for `'static` string literals.
#[macro_export]
macro_rules! sformat {
    ($fmt:expr) => {
        $crate::strings::SharedString::from_fmt(::std::format_args!($fmt))
    };

    ($fmt:expr, $($args:tt)*) => {
        $crate::strings::SharedString::from_fmt(::std::format_args!($fmt, $($args)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macro_works() {
        assert!(sformat!("").is_empty());
        assert_eq!(
            sformat!("Hello World"),
            SharedString::from_static_str("Hello World")
        );
        assert_eq!(
            sformat!("Hello {}", "World"),
            SharedString::from_static_str("Hello World")
        );
    }
}
