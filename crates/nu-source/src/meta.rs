use crate::pretty::{b, DebugDocBuilder, PrettyDebugWithSource};
use crate::text::Text;
use crate::tracable::TracableContext;

use derive_new::new;
use getset::Getters;
use serde::Deserialize;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnchorLocation {
    Url(String),
    File(String),
    Source(Text),
}

pub trait HasTag {
    fn tag(&self) -> Tag;
}

#[derive(new, Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Hash)]
pub struct Spanned<T> {
    pub span: Span,
    pub item: T,
}

impl<T> Spanned<T> {
    pub fn map<U>(self, input: impl FnOnce(T) -> U) -> Spanned<U> {
        let span = self.span;

        let mapped = input(self.item);
        mapped.spanned(span)
    }
}

impl Spanned<String> {
    pub fn items<'a, U>(
        items: impl Iterator<Item = &'a Spanned<String>>,
    ) -> impl Iterator<Item = &'a str> {
        items.map(|item| &item.item[..])
    }
}

impl Spanned<String> {
    pub fn borrow_spanned(&self) -> Spanned<&str> {
        let span = self.span;
        self.item[..].spanned(span)
    }
}

pub trait SpannedItem: Sized {
    fn spanned(self, span: impl Into<Span>) -> Spanned<Self> {
        Spanned {
            item: self,
            span: span.into(),
        }
    }

    fn spanned_unknown(self) -> Spanned<Self> {
        Spanned {
            item: self,
            span: Span::unknown(),
        }
    }
}
impl<T> SpannedItem for T {}

impl<T> std::ops::Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.item
    }
}

#[derive(new, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Hash)]
pub struct Tagged<T> {
    pub tag: Tag,
    pub item: T,
}

impl Tagged<String> {
    pub fn borrow_spanned(&self) -> Spanned<&str> {
        let span = self.tag.span;
        self.item[..].spanned(span)
    }

    pub fn borrow_tagged(&self) -> Tagged<&str> {
        self.item[..].tagged(self.tag.clone())
    }
}

impl<T> Tagged<Vec<T>> {
    pub fn items(&self) -> impl Iterator<Item = &T> {
        self.item.iter()
    }
}

impl<T> HasTag for Tagged<T> {
    fn tag(&self) -> Tag {
        self.tag.clone()
    }
}

impl AsRef<Path> for Tagged<PathBuf> {
    fn as_ref(&self) -> &Path {
        self.item.as_ref()
    }
}

pub trait TaggedItem: Sized {
    fn tagged(self, tag: impl Into<Tag>) -> Tagged<Self> {
        Tagged {
            item: self,
            tag: tag.into(),
        }
    }

    // For now, this is a temporary facility. In many cases, there are other useful spans that we
    // could be using, such as the original source spans of JSON or Toml files, but we don't yet
    // have the infrastructure to make that work.
    fn tagged_unknown(self) -> Tagged<Self> {
        Tagged {
            item: self,
            tag: Tag {
                span: Span::unknown(),
                anchor: None,
            },
        }
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
    pub fn map<U>(self, input: impl FnOnce(T) -> U) -> Tagged<U> {
        let tag = self.tag();

        let mapped = input(self.item);
        mapped.tagged(tag)
    }

    pub fn map_anchored(self, anchor: &Option<AnchorLocation>) -> Tagged<T> {
        let mut tag = self.tag;

        tag.anchor = anchor.clone();

        Tagged {
            item: self.item,
            tag,
        }
    }

    pub fn transpose(&self) -> Tagged<&T> {
        Tagged {
            item: &self.item,
            tag: self.tag.clone(),
        }
    }

    pub fn tag(&self) -> Tag {
        self.tag.clone()
    }

    pub fn span(&self) -> Span {
        self.tag.span
    }

    pub fn anchor(&self) -> Option<AnchorLocation> {
        self.tag.anchor.clone()
    }

    pub fn anchor_name(&self) -> Option<String> {
        match self.tag.anchor {
            Some(AnchorLocation::File(ref file)) => Some(file.clone()),
            Some(AnchorLocation::Url(ref url)) => Some(url.clone()),
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
        input.clone()
    }
}

impl<T> From<nom_locate::LocatedSpanEx<&str, T>> for Span {
    fn from(input: nom_locate::LocatedSpanEx<&str, T>) -> Span {
        Span::new(input.offset, input.offset + input.fragment.len())
    }
}

impl<T>
    From<(
        nom_locate::LocatedSpanEx<T, u64>,
        nom_locate::LocatedSpanEx<T, u64>,
    )> for Span
{
    fn from(
        input: (
            nom_locate::LocatedSpanEx<T, u64>,
            nom_locate::LocatedSpanEx<T, u64>,
        ),
    ) -> Span {
        Span::new(input.0.offset, input.1.offset)
    }
}

impl From<(usize, usize)> for Span {
    fn from(input: (usize, usize)) -> Span {
        Span::new(input.0, input.1)
    }
}

impl From<&std::ops::Range<usize>> for Span {
    fn from(input: &std::ops::Range<usize>) -> Span {
        Span::new(input.start, input.end)
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize, Hash, Getters, new,
)]
pub struct Tag {
    pub anchor: Option<AnchorLocation>,
    pub span: Span,
}

impl From<Span> for Tag {
    fn from(span: Span) -> Self {
        Tag { anchor: None, span }
    }
}

impl From<&Span> for Tag {
    fn from(span: &Span) -> Self {
        Tag {
            anchor: None,
            span: *span,
        }
    }
}

impl From<(usize, usize, TracableContext)> for Tag {
    fn from((start, end, _context): (usize, usize, TracableContext)) -> Self {
        Tag {
            anchor: None,
            span: Span::new(start, end),
        }
    }
}

impl From<(usize, usize, AnchorLocation)> for Tag {
    fn from((start, end, anchor): (usize, usize, AnchorLocation)) -> Self {
        Tag {
            anchor: Some(anchor),
            span: Span::new(start, end),
        }
    }
}

impl From<(usize, usize, Option<AnchorLocation>)> for Tag {
    fn from((start, end, anchor): (usize, usize, Option<AnchorLocation>)) -> Self {
        Tag {
            anchor,
            span: Span::new(start, end),
        }
    }
}

impl From<nom_locate::LocatedSpanEx<&str, TracableContext>> for Tag {
    fn from(input: nom_locate::LocatedSpanEx<&str, TracableContext>) -> Tag {
        Tag {
            anchor: None,
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
        Tag { anchor: None, span }
    }

    pub fn for_char(pos: usize, anchor: AnchorLocation) -> Tag {
        Tag {
            anchor: Some(anchor),
            span: Span::new(pos, pos + 1),
        }
    }

    pub fn unknown_span(anchor: AnchorLocation) -> Tag {
        Tag {
            anchor: Some(anchor),
            span: Span::unknown(),
        }
    }

    pub fn unknown() -> Tag {
        Tag {
            anchor: None,
            span: Span::unknown(),
        }
    }

    pub fn anchor(&self) -> Option<AnchorLocation> {
        self.anchor.clone()
    }

    pub fn until(&self, other: impl Into<Tag>) -> Tag {
        let other = other.into();
        debug_assert!(
            self.anchor == other.anchor,
            "Can only merge two tags with the same anchor"
        );

        Tag {
            span: Span::new(self.span.start, other.span.end),
            anchor: self.anchor.clone(),
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
                    anchor: self.anchor.clone(),
                }
            }
            None => self.clone(),
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

    pub fn anchor_name(&self) -> Option<String> {
        match self.anchor {
            Some(AnchorLocation::File(ref file)) => Some(file.clone()),
            Some(AnchorLocation::Url(ref url)) => Some(url.clone()),
            _ => None,
        }
    }
}

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

pub fn span_for_spanned_list(mut iter: impl Iterator<Item = Span>) -> Span {
    let first = iter.next();

    let first = match first {
        None => return Span::unknown(),
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

impl From<&Span> for Span {
    fn from(span: &Span) -> Span {
        *span
    }
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

    pub fn for_char(pos: usize) -> Span {
        Span {
            start: pos,
            end: pos + 1,
        }
    }

    pub fn since(&self, other: impl Into<Span>) -> Span {
        let other = other.into();

        Span::new(other.start, self.end)
    }

    pub fn until(&self, other: impl Into<Span>) -> Span {
        let other = other.into();

        Span::new(self.start, other.end)
    }

    pub fn until_option(&self, other: Option<impl Into<Span>>) -> Span {
        match other {
            Some(other) => {
                let other = other.into();

                Span::new(self.start, other.end)
            }
            None => *self,
        }
    }

    pub fn string<'a>(&self, source: &'a str) -> String {
        self.slice(source).to_string()
    }

    pub fn spanned_slice<'a>(&self, source: &'a str) -> Spanned<&'a str> {
        self.slice(source).spanned(*self)
    }

    pub fn spanned_string<'a>(&self, source: &'a str) -> Spanned<String> {
        self.slice(source).to_string().spanned(*self)
    }

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
        if self.end < start {
            Span::new(start, start)
        } else {
            Span::new(start, self.end)
        }
    }

    fn with_end(&self, end: usize) -> Self {
        if end < self.start {
            Span::new(end, end)
        } else {
            Span::new(self.start, end)
        }
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}

pub trait HasSpan: PrettyDebugWithSource {
    fn span(&self) -> Span;
}

pub trait HasFallibleSpan: PrettyDebugWithSource {
    fn maybe_span(&self) -> Option<Span>;
}

impl<T: HasSpan> HasFallibleSpan for T {
    fn maybe_span(&self) -> Option<Span> {
        Some(HasSpan::span(self))
    }
}

impl<T> HasSpan for Spanned<T>
where
    Spanned<T>: PrettyDebugWithSource,
{
    fn span(&self) -> Span {
        self.span
    }
}

impl PrettyDebugWithSource for Option<Span> {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self {
            None => b::description("no span"),
            Some(span) => span.pretty_debug(source),
        }
    }
}

impl HasFallibleSpan for Option<Span> {
    fn maybe_span(&self) -> Option<Span> {
        *self
    }
}

impl PrettyDebugWithSource for Span {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::typed(
            "spanned",
            b::keyword("for") + b::space() + b::description(format!("{:?}", source)),
        )
    }
}

impl HasSpan for Span {
    fn span(&self) -> Span {
        *self
    }
}

impl<T> PrettyDebugWithSource for Option<Spanned<T>>
where
    Spanned<T>: PrettyDebugWithSource,
{
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self {
            None => b::description("nothing"),
            Some(v) => v.pretty_debug(source),
        }
    }
}

impl<T> HasFallibleSpan for Option<Spanned<T>>
where
    Spanned<T>: PrettyDebugWithSource,
{
    fn maybe_span(&self) -> Option<Span> {
        match self {
            None => None,
            Some(value) => Some(value.span),
        }
    }
}

impl<T> PrettyDebugWithSource for Option<Tagged<T>>
where
    Tagged<T>: PrettyDebugWithSource,
{
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self {
            None => b::description("nothing"),
            Some(d) => d.pretty_debug(source),
        }
    }
}

impl<T> HasFallibleSpan for Option<Tagged<T>>
where
    Tagged<T>: PrettyDebugWithSource,
{
    fn maybe_span(&self) -> Option<Span> {
        match self {
            None => None,
            Some(value) => Some(value.tag.span),
        }
    }
}

impl<T> HasSpan for Tagged<T>
where
    Tagged<T>: PrettyDebugWithSource,
{
    fn span(&self) -> Span {
        self.tag.span
    }
}
