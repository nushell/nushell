use crate::prelude::*;
use crate::Text;
use derive_new::new;
use getset::Getters;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

#[derive(new, Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Hash)]
pub struct Tagged<T> {
    pub tag: Tag,
    pub item: T,
}

impl<T> HasSpan for Tagged<T> {
    fn span(&self) -> Span {
        self.tag.span
    }
}

pub trait TaggedItem: Sized {
    fn tagged(self, tag: impl Into<Tag>) -> Tagged<Self> {
        Tagged::from_item(self, tag.into())
    }

    fn simple_spanned(self, span: impl Into<Span>) -> Tagged<Self> {
        Tagged::from_simple_spanned_item(self, span.into())
    }

    // For now, this is a temporary facility. In many cases, there are other useful spans that we
    // could be using, such as the original source spans of JSON or Toml files, but we don't yet
    // have the infrastructure to make that work.
    fn tagged_unknown(self) -> Tagged<Self> {
        Tagged::from_item(
            self,
            Tag {
                span: Span::unknown(),
                origin: None,
            },
        )
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
    pub fn spanned(self, span: impl Into<Span>) -> Tagged<T> {
        Tagged::from_item(
            self.item,
            Tag {
                span: span.into(),
                origin: None,
            },
        )
    }

    pub fn from_item(item: T, tag: impl Into<Tag>) -> Tagged<T> {
        Tagged {
            item,
            tag: tag.into(),
        }
    }

    pub fn from_simple_spanned_item(item: T, span: impl Into<Span>) -> Tagged<T> {
        Tagged::from_item(
            item,
            Tag {
                span: span.into(),
                origin: None,
            },
        )
    }

    pub fn map<U>(self, input: impl FnOnce(T) -> U) -> Tagged<U> {
        let tag = self.tag();

        let mapped = input(self.item);
        Tagged::from_item(mapped, tag.clone())
    }

    crate fn copy_span<U>(&self, output: U) -> Tagged<U> {
        let span = self.span();

        Tagged::from_simple_spanned_item(output, span)
    }

    pub fn source(&self, source: &Text) -> Text {
        Text::from(self.span().slice(source))
    }

    pub fn span(&self) -> Span {
        self.tag.span
    }

    pub fn tag(&self) -> Tag {
        self.tag
    }

    pub fn origin(&self) -> Option<uuid::Uuid> {
        self.tag.origin
    }

    pub fn item(&self) -> &T {
        &self.item
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
        }
    }
}

impl<T> From<(nom5_locate::LocatedSpan<T>, nom5_locate::LocatedSpan<T>)> for Span {
    fn from(input: (nom5_locate::LocatedSpan<T>, nom5_locate::LocatedSpan<T>)) -> Span {
        Span {
            start: input.0.offset,
            end: input.1.offset,
        }
    }
}

impl From<(usize, usize)> for Span {
    fn from(input: (usize, usize)) -> Span {
        Span {
            start: input.0,
            end: input.1,
        }
    }
}

impl From<&std::ops::Range<usize>> for Span {
    fn from(input: &std::ops::Range<usize>) -> Span {
        Span {
            start: input.start,
            end: input.end,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize, Hash, Getters,
)]
pub struct Tag {
    pub origin: Option<Uuid>,
    pub span: Span,
}

impl Tag {
    pub fn unknown_origin(span: Span) -> Tag {
        Tag { origin: None, span }
    }

    pub fn unknown() -> Tag {
        Tag {
            origin: None,
            span: Span::unknown(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize, Hash)]
pub struct Span {
    crate start: usize,
    crate end: usize,
}

impl From<Option<Span>> for Span {
    fn from(input: Option<Span>) -> Span {
        match input {
            None => Span { start: 0, end: 0 },
            Some(span) => span,
        }
    }
}

impl Span {
    pub fn unknown() -> Span {
        Span { start: 0, end: 0 }
    }

    /*
    pub fn unknown_with_uuid(uuid: Uuid) -> Span {
        Span {
            start: 0,
            end: 0,
            source: Some(uuid),
        }
    }
    */

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
        }
    }

    fn with_end(&self, end: usize) -> Self {
        Span {
            start: self.start,
            end,
        }
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}
