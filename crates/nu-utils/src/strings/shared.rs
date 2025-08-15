use std::{
    borrow::Borrow,
    fmt::{Arguments, Debug, Display},
    hash::Hash,
    ops::Deref,
};

pub struct SharedString(lean_string::LeanString);

const _: () = const {
    assert!(size_of::<SharedString>() == size_of::<[usize; 2]>());
    assert!(size_of::<SharedString>() == size_of::<Option<SharedString>>());
};

impl SharedString {
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
        Self(lean_string::LeanString::from(string))
    }

    #[inline]
    pub fn from_static_str(string: &'static str) -> Self {
        Self(lean_string::LeanString::from_static_str(string))
    }

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
        self.0.partial_cmp(&other.0)
    }
}

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
        assert_eq!(sformat!("Hello World"), SharedString::from_static_str("Hello World"));
        assert_eq!(sformat!("Hello {}", "World"), SharedString::from_static_str("Hello World"));
    }
}
