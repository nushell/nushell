use crate::Text;
use derive_new::new;
use getset::Getters;
use serde::Serialize;
use serde_derive::Deserialize;
use uuid::Uuid;

#[derive(
    new, Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Hash, Getters,
)]
#[get = "crate"]
pub struct Tagged<T> {
    pub tag: Tag,
    pub item: T,
}

pub trait TaggedItem: Sized {
    fn tagged(self, span: impl Into<Span>) -> Tagged<Self> {
        Tagged::from_item(self, span.into())
    }

    // For now, this is a temporary facility. In many cases, there are other useful spans that we
    // could be using, such as the original source spans of JSON or Toml files, but we don't yet
    // have the infrastructure to make that work.
    fn tagged_unknown(self) -> Tagged<Self> {
        Tagged::from_item(self, (0, 0))
    }
}

impl<T> TaggedItem for T {}

impl<T> std::ops::Deref for Tagged<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.item
    }
}

impl<T> Tagged<T> {
    pub fn tagged(self, span: impl Into<Span>) -> Tagged<T> {
        Tagged::from_item(self.item, span.into())
    }

    pub fn from_item(item: T, span: impl Into<Span>) -> Tagged<T> {
        Tagged {
            item,
            tag: Tag { span: span.into() },
        }
    }

    pub fn map<U>(self, input: impl FnOnce(T) -> U) -> Tagged<U> {
        let span = self.span();

        let mapped = input(self.item);
        Tagged::from_item(mapped, span)
    }

    crate fn copy_span<U>(&self, output: U) -> Tagged<U> {
        let span = self.span();

        Tagged::from_item(output, span)
    }

    pub fn source(&self, source: &Text) -> Text {
        Text::from(self.span().slice(source))
    }

    pub fn span(&self) -> Span {
        self.tag.span
    }
}

impl<T> From<&Tagged<T>> for Span {
    fn from(input: &Tagged<T>) -> Span {
        input.span()
    }
}

impl From<&Span> for Span {
    fn from(input: &Span) -> Span {
        *input
    }
}

impl From<nom5_locate::LocatedSpan<&str>> for Span {
    fn from(input: nom5_locate::LocatedSpan<&str>) -> Span {
        Span {
            start: input.offset,
            end: input.offset + input.fragment.len(),
            source: None,
        }
    }
}

impl<T> From<(nom5_locate::LocatedSpan<T>, nom5_locate::LocatedSpan<T>)> for Span {
    fn from(input: (nom5_locate::LocatedSpan<T>, nom5_locate::LocatedSpan<T>)) -> Span {
        Span {
            start: input.0.offset,
            end: input.1.offset,
            source: None,
        }
    }
}

impl From<(usize, usize)> for Span {
    fn from(input: (usize, usize)) -> Span {
        Span {
            start: input.0,
            end: input.1,
            source: None,
        }
    }
}

impl From<&std::ops::Range<usize>> for Span {
    fn from(input: &std::ops::Range<usize>) -> Span {
        Span {
            start: input.start,
            end: input.end,
            source: None,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize, Hash, Getters,
)]
pub struct Tag {
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize, Hash)]
pub struct Span {
    crate start: usize,
    crate end: usize,
    pub source: Option<Uuid>,
}

impl From<Option<Span>> for Span {
    fn from(input: Option<Span>) -> Span {
        match input {
            None => Span {
                start: 0,
                end: 0,
                source: None,
            },
            Some(span) => span,
        }
    }
}

impl Span {
    pub fn unknown() -> Span {
        Span {
            start: 0,
            end: 0,
            source: None,
        }
    }

    pub fn unknown_with_uuid(uuid: Uuid) -> Span {
        Span {
            start: 0,
            end: 0,
            source: Some(uuid),
        }
    }

    pub fn is_unknown(&self) -> bool {
        self.start == 0 && self.end == 0
    }

    pub fn slice(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }
}

impl language_reporting::ReportingSpan for Span {
    fn with_start(&self, start: usize) -> Self {
        Span {
            start,
            end: self.end,
            source: None,
        }
    }

    fn with_end(&self, end: usize) -> Self {
        Span {
            start: self.start,
            end,
            source: None,
        }
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}
