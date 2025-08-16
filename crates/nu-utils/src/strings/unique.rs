use std::{
    borrow::Borrow,
    fmt::{Arguments, Debug, Display},
    hash::Hash,
    ops::Deref,
};

use serde::{Deserialize, Serialize};

/// An owned, immutable string with compact storage.
///
/// `UniqueString` is designed for immutable strings that are not frequently cloned and hold ownership.
/// It offers similar characteristics to `Box<str>` but with several key
/// optimizations for improved efficiency and memory usage:
///
/// - **Efficient for Unique Strings:**
///   When strings are not frequently cloned, `UniqueString` can be more performant than
///   reference-counted alternatives like [`SharedString`](super::SharedString) as it avoids the
///   overhead of atomic reference counting.
///
/// - **Small String Optimization (SSO):**
///   For shorter strings, the data is stored directly within the `UniqueString` struct, keeping
///   the data on the stack and avoiding indirection.
///
/// - **Static String Re-use:**
///   Strings with a `'static` lifetime are directly referenced, avoiding unnecessary copies or
///   allocations.
///
/// - **Niche Optimization:**
///   `UniqueString` allows niche-optimization, meaning that [`Option<UniqueString>`] has the same
///   memory footprint as `UniqueString`.
///
/// - **Compact Size:**
///   On 64-bit systems, `UniqueString` is 16 bytes.
///   This is achieved by disregarding the capacity of a `String` since we only hold the string as
///   immutable.
///
/// Internally, `UniqueString` is powered by [`byteyarn::Yarn`], which provides the
/// underlying implementation for these optimizations.
pub struct UniqueString(byteyarn::Yarn);

const _: () = const {
    assert!(size_of::<UniqueString>() == size_of::<[usize; 2]>());
    assert!(size_of::<UniqueString>() == size_of::<Option<UniqueString>>());
};

impl UniqueString {
    /// Returns a string slice containing the entire `UniqueString`.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Returns a byte slice of this `UniqueString`'s contents.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Returns the length of this `UniqueString`, in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the `UniqueString` has a length of 0, `false` otherwise.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns a `UniqueString` by taking ownership of an allocation.
    #[inline]
    pub fn from_string(string: String) -> Self {
        Self(byteyarn::Yarn::from_string(string))
    }

    /// Returns a `UniqueString` pointing to the given slice, without copying.
    ///
    /// By using this function instead of [`from_string`](Self::from_string), we can avoid any
    /// copying and always refer to the provided static string slice.
    #[inline]
    pub fn from_static_str(string: &'static str) -> Self {
        Self(byteyarn::Yarn::from_static(string))
    }

    /// Builds a new `UniqueString` from the given formatting arguments.
    ///
    /// You can get an [`Arguments`] instance by calling [`format_args!`].
    /// This function is used when using [`uformat!`](crate::uformat) instead of [`format!`] to
    /// create a `UniqueString`.
    #[inline]
    pub fn from_fmt(arguments: Arguments) -> Self {
        Self(byteyarn::Yarn::from_fmt(arguments))
    }
}

impl AsRef<str> for UniqueString {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for UniqueString {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Clone for UniqueString {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Debug for UniqueString {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Default for UniqueString {
    #[inline]
    fn default() -> Self {
        Self(Default::default())
    }
}

impl Deref for UniqueString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl Display for UniqueString {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Eq for UniqueString {}

impl From<String> for UniqueString {
    #[inline]
    fn from(string: String) -> Self {
        Self::from_string(string)
    }
}

impl From<&'static str> for UniqueString {
    #[inline]
    fn from(string: &'static str) -> Self {
        Self::from_static_str(string)
    }
}

impl Hash for UniqueString {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Ord for UniqueString {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialEq for UniqueString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd for UniqueString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Serialize for UniqueString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for UniqueString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from_string(s))
    }
}

/// A macro for creating [`UniqueString`] instances from format arguments.
///
/// This macro works similarly to [`format!`] but returns a [`UniqueString`] instead of a [`String`].
/// It attempts to optimize for `'static` string literals.
#[macro_export]
macro_rules! uformat {
    ($fmt:expr) => {
        $crate::strings::UniqueString::from_fmt(::std::format_args!($fmt))
    };

    ($fmt:expr, $($args:tt)*) => {
        $crate::strings::UniqueString::from_fmt(::std::format_args!($fmt, $($args)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macro_works() {
        assert!(uformat!("").is_empty());
        assert_eq!(
            uformat!("Hello World"),
            UniqueString::from_static_str("Hello World")
        );
        assert_eq!(
            uformat!("Hello {}", "World"),
            UniqueString::from_static_str("Hello World")
        );
    }
}
