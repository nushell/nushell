use crate::context::{AnchorLocation, SourceMap};
use crate::parser::parse::parser::TracableContext;
use crate::prelude::*;
use crate::Text;
use derive_new::new;
use getset::Getters;
use serde::Deserialize;
use serde::Serialize;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(new, Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Hash)]
pub struct Tagged<T> {
    pub tag: Tag,
    pub item: T,
}

impl<T> HasTag for Tagged<T> {
    fn tag(&self) -> Tag {
        self.tag
    }
}

impl AsRef<Path> for Tagged<PathBuf> {
    fn as_ref(&self) -> &Path {
        self.item.as_ref()
    }
}

pub trait TaggedItem: Sized {
    fn tagged(self, tag: impl Into<Tag>) -> Tagged<Self> {
        Tagged::from_item(self, tag.into())
    }

    // For now, this is a temporary facility. In many cases, there are other useful spans that we
    // could be using, such as the original source spans of JSON or Toml files, but we don't yet
    // have the infrastructure to make that work.
    fn tagged_unknown(self) -> Tagged<Self> {
        Tagged::from_item(
            self,
            Tag {
                span: Span::unknown(),
                anchor: uuid::Uuid::nil(),
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
    pub fn with_tag(self, tag: impl Into<Tag>) -> Tagged<T> {
        Tagged::from_item(self.item, tag)
    }

    pub fn from_item(item: T, tag: impl Into<Tag>) -> Tagged<T> {
        Tagged {
            item,
            tag: tag.into(),
        }
    }

    pub fn map<U>(self, input: impl FnOnce(T) -> U) -> Tagged<U> {
        let tag = self.tag();

        let mapped = input(self.item);
        Tagged::from_item(mapped, tag)
    }

    pub(crate) fn copy_tag<U>(&self, output: U) -> Tagged<U> {
        Tagged::from_item(output, self.tag())
    }

    pub fn source(&self, source: &Text) -> Text {
        Text::from(self.tag().slice(source))
    }

    pub fn tag(&self) -> Tag {
        self.tag
    }

    pub fn span(&self) -> Span {
        self.tag.span
    }

    pub fn anchor(&self) -> uuid::Uuid {
        self.tag.anchor
    }

    pub fn anchor_name(&self, source_map: &SourceMap) -> Option<String> {
        match source_map.get(&self.tag.anchor) {
            Some(AnchorLocation::File(file)) => Some(file.clone()),
            Some(AnchorLocation::Url(url)) => Some(url.clone()),
            _ => None,
        }
    }

    pub fn item(&self) -> &T {
        &self.item
    }

    pub fn into_parts(self) -> (T, Tag) {
        (self.item, self.tag)
    }
}

impl From<&Tag> for Tag {
    fn from(input: &Tag) -> Tag {
        *input
    }
}

impl From<nom_locate::LocatedSpanEx<&str, Uuid>> for Span {
    fn from(input: nom_locate::LocatedSpanEx<&str, Uuid>) -> Span {
        Span::new(input.offset, input.offset + input.fragment.len())
    }
}

impl<T>
    From<(
        nom_locate::LocatedSpanEx<T, Uuid>,
        nom_locate::LocatedSpanEx<T, Uuid>,
    )> for Span
{
    fn from(
        input: (
            nom_locate::LocatedSpanEx<T, Uuid>,
            nom_locate::LocatedSpanEx<T, Uuid>,
        ),
    ) -> Span {
        Span {
            start: input.0.offset,
            end: input.1.offset,
        }
    }
}

impl From<(usize, usize)> for Span {
    fn from(input: (usize, usize)) -> Span {
        Span::new(input.0, input.1)
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
    Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize, Hash, Getters, new,
)]
pub struct Tag {
    pub anchor: Uuid,
    pub span: Span,
}

impl From<Span> for Tag {
    fn from(span: Span) -> Self {
        Tag {
            anchor: uuid::Uuid::nil(),
            span,
        }
    }
}

impl From<&Span> for Tag {
    fn from(span: &Span) -> Self {
        Tag {
            anchor: uuid::Uuid::nil(),
            span: *span,
        }
    }
}

impl From<(usize, usize, TracableContext)> for Tag {
    fn from((start, end, context): (usize, usize, TracableContext)) -> Self {
        Tag {
            anchor: context.origin,
            span: Span::new(start, end),
        }
    }
}

impl From<(usize, usize, Uuid)> for Tag {
    fn from((start, end, anchor): (usize, usize, Uuid)) -> Self {
        Tag {
            anchor,
            span: Span::new(start, end),
        }
    }
}

impl From<(usize, usize, Option<Uuid>)> for Tag {
    fn from((start, end, anchor): (usize, usize, Option<Uuid>)) -> Self {
        Tag {
            anchor: anchor.unwrap_or(uuid::Uuid::nil()),
            span: Span::new(start, end),
        }
    }
}

impl From<nom_locate::LocatedSpanEx<&str, TracableContext>> for Tag {
    fn from(input: nom_locate::LocatedSpanEx<&str, TracableContext>) -> Tag {
        Tag {
            anchor: input.extra.origin,
            span: Span::new(input.offset, input.offset + input.fragment.len()),
        }
    }
}

impl From<Tag> for Span {
    fn from(tag: Tag) -> Self {
        tag.span
    }
}

impl From<&Tag> for Span {
    fn from(tag: &Tag) -> Self {
        tag.span
    }
}

impl Tag {
    pub fn unknown_anchor(span: Span) -> Tag {
        Tag {
            anchor: uuid::Uuid::nil(),
            span,
        }
    }

    pub fn for_char(pos: usize, anchor: Uuid) -> Tag {
        Tag {
            anchor,
            span: Span {
                start: pos,
                end: pos + 1,
            },
        }
    }

    pub fn unknown_span(anchor: Uuid) -> Tag {
        Tag {
            anchor,
            span: Span::unknown(),
        }
    }

    pub fn unknown() -> Tag {
        Tag {
            anchor: uuid::Uuid::nil(),
            span: Span::unknown(),
        }
    }

    pub fn until(&self, other: impl Into<Tag>) -> Tag {
        let other = other.into();
        debug_assert!(
            self.anchor == other.anchor,
            "Can only merge two tags with the same anchor"
        );

        Tag {
            span: Span::new(self.span.start, other.span.end),
            anchor: self.anchor,
        }
    }

    pub fn until_option(&self, other: Option<impl Into<Tag>>) -> Tag {
        match other {
            Some(other) => {
                let other = other.into();
                debug_assert!(
                    self.anchor == other.anchor,
                    "Can only merge two tags with the same anchor"
                );

                Tag {
                    span: Span::new(self.span.start, other.span.end),
                    anchor: self.anchor,
                }
            }
            None => *self,
        }
    }

    pub fn slice<'a>(&self, source: &'a str) -> &'a str {
        self.span.slice(source)
    }

    pub fn string<'a>(&self, source: &'a str) -> String {
        self.span.slice(source).to_string()
    }

    pub fn tagged_slice<'a>(&self, source: &'a str) -> Tagged<&'a str> {
        self.span.slice(source).tagged(self)
    }

    pub fn tagged_string<'a>(&self, source: &'a str) -> Tagged<String> {
        self.span.slice(source).to_string().tagged(self)
    }
}

#[allow(unused)]
pub fn tag_for_tagged_list(mut iter: impl Iterator<Item = Tag>) -> Tag {
    let first = iter.next();

    let first = match first {
        None => return Tag::unknown(),
        Some(first) => first,
    };

    let last = iter.last();

    match last {
        None => first,
        Some(last) => first.until(last),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize, Hash)]
pub struct Span {
    start: usize,
    end: usize,
}

impl From<Option<Span>> for Span {
    fn from(input: Option<Span>) -> Span {
        match input {
            None => Span::new(0, 0),
            Some(span) => span,
        }
    }
}

impl Span {
    pub fn unknown() -> Span {
        Span::new(0, 0)
    }

    pub fn new(start: usize, end: usize) -> Span {
        assert!(
            end >= start,
            "Can't create a Span whose end < start, start={}, end={}",
            start,
            end
        );

        Span { start, end }
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

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn is_unknown(&self) -> bool {
        self.start == 0 && self.end == 0
    }

    pub fn slice<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }
}

impl language_reporting::ReportingSpan for Span {
    fn with_start(&self, start: usize) -> Self {
        Span::new(start, self.end)
    }

    fn with_end(&self, end: usize) -> Self {
        Span::new(self.start, end)
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}

impl language_reporting::ReportingSpan for Tag {
    fn with_start(&self, start: usize) -> Self {
        Tag {
            span: Span::new(start, self.span.end),
            anchor: self.anchor,
        }
    }

    fn with_end(&self, end: usize) -> Self {
        Tag {
            span: Span::new(self.span.start, end),
            anchor: self.anchor,
        }
    }

    fn start(&self) -> usize {
        self.span.start
    }

    fn end(&self) -> usize {
        self.span.end
    }
}
