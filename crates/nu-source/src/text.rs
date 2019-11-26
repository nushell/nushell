use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::Range;
use std::sync::Arc;

/// A "Text" is like a string except that it can be cheaply cloned.
/// You can also "extract" subtexts quite cheaply. You can also deref
/// an `&Text` into a `&str` for interoperability.
///
/// Used to represent the value of an input file.
#[derive(Clone)]
pub struct Text {
    text: Arc<String>,
    start: usize,
    end: usize,
}

impl Text {
    /// Modifies this restrict to a subset of its current range.
    pub fn select(&mut self, range: Range<usize>) {
        let len = range.end - range.start;
        let new_start = self.start + range.start;
        let new_end = new_start + len;
        assert!(new_end <= self.end);

        self.start = new_start;
        self.end = new_end;
    }

    /// Extract a new `Text` that is a subset of an old `Text`
    /// -- `text.extract(1..3)` is similar to `&foo[1..3]` except that
    /// it gives back an owned value instead of a borrowed value.
    pub fn slice(&self, range: Range<usize>) -> Self {
        let mut result = self.clone();
        result.select(range);
        result
    }
}

impl From<Arc<String>> for Text {
    fn from(text: Arc<String>) -> Self {
        let end = text.len();
        Self {
            text,
            start: 0,
            end,
        }
    }
}

impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        &*self
    }
}

impl From<String> for Text {
    fn from(text: String) -> Self {
        Text::from(Arc::new(text))
    }
}

impl From<&String> for Text {
    fn from(text: &String) -> Self {
        Text::from(text.to_string())
    }
}

impl From<&str> for Text {
    fn from(text: &str) -> Self {
        Text::from(text.to_string())
    }
}

impl From<&Text> for Text {
    fn from(text: &Text) -> Self {
        text.clone()
    }
}

impl std::borrow::Borrow<str> for Text {
    fn borrow(&self) -> &str {
        &*self
    }
}

impl std::ops::Deref for Text {
    type Target = str;

    fn deref(&self) -> &str {
        &self.text[self.start..self.end]
    }
}

impl std::fmt::Display for Text {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <str as std::fmt::Display>::fmt(self, fmt)
    }
}

impl std::fmt::Debug for Text {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <str as std::fmt::Debug>::fmt(self, fmt)
    }
}

impl PartialEq<Text> for Text {
    fn eq(&self, other: &Text) -> bool {
        let this: &str = self;
        let other: &str = other;
        this == other
    }
}

impl Eq for Text {}

impl PartialEq<str> for Text {
    fn eq(&self, other: &str) -> bool {
        let this: &str = self;
        this == other
    }
}

impl PartialEq<String> for Text {
    fn eq(&self, other: &String) -> bool {
        let this: &str = self;
        let other: &str = other;
        this == other
    }
}

impl PartialEq<Text> for str {
    fn eq(&self, other: &Text) -> bool {
        other == self
    }
}

impl PartialEq<Text> for String {
    fn eq(&self, other: &Text) -> bool {
        other == self
    }
}

impl<T: ?Sized> PartialEq<&T> for Text
where
    Text: PartialEq<T>,
{
    fn eq(&self, other: &&T) -> bool {
        self == *other
    }
}

impl Hash for Text {
    fn hash<H: Hasher>(&self, state: &mut H) {
        <str as Hash>::hash(self, state)
    }
}

impl PartialOrd<Text> for Text {
    fn partial_cmp(&self, other: &Text) -> Option<Ordering> {
        let this: &str = self;
        let other: &str = other;
        this.partial_cmp(other)
    }
}

impl Ord for Text {
    fn cmp(&self, other: &Text) -> Ordering {
        let this: &str = self;
        let other: &str = other;
        this.cmp(other)
    }
}

impl PartialOrd<str> for Text {
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        let this: &str = self;
        this.partial_cmp(other)
    }
}

impl PartialOrd<String> for Text {
    fn partial_cmp(&self, other: &String) -> Option<Ordering> {
        let this: &str = self;
        let other: &str = other;
        this.partial_cmp(other)
    }
}

impl PartialOrd<Text> for str {
    fn partial_cmp(&self, other: &Text) -> Option<Ordering> {
        other.partial_cmp(self)
    }
}

impl PartialOrd<Text> for String {
    fn partial_cmp(&self, other: &Text) -> Option<Ordering> {
        other.partial_cmp(self)
    }
}

impl<T: ?Sized> PartialOrd<&T> for Text
where
    Text: PartialOrd<T>,
{
    fn partial_cmp(&self, other: &&T) -> Option<Ordering> {
        self.partial_cmp(*other)
    }
}

impl Serialize for Text {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_ref().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Text {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Text::from(String::deserialize(deserializer)?))
    }
}
