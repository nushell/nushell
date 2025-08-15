use std::{
    borrow::Borrow,
    fmt::{Arguments, Debug, Display},
    hash::Hash,
    ops::Deref,
};

pub struct UniqueString(byteyarn::Yarn);

const _: () = const {
    assert!(size_of::<UniqueString>() == size_of::<[usize; 2]>());
    assert!(size_of::<UniqueString>() == size_of::<Option<UniqueString>>());
};

impl UniqueString {
    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn from_string(string: String) -> Self {
        Self(byteyarn::Yarn::from_string(string))
    }

    #[inline]
    pub fn from_static_str(string: &'static str) -> Self {
        Self(byteyarn::Yarn::from_static(string))
    }

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
        self.0.partial_cmp(&other.0)
    }
}

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
