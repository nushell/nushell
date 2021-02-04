use crate::pretty::{DbgDocBldr, DebugDocBuilder, PrettyDebugWithSource};
use crate::text::Text;

use derive_new::new;
use getset::Getters;
use serde::Deserialize;
use serde::Serialize;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

/// Anchors represent a location that a value originated from. The value may have been loaded from a file, fetched from a website, or parsed from some text
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnchorLocation {
    /// The originating site where the value was first found
    Url(String),
    /// The original file where the value was loaded from
    File(String),
    /// The text where the value was parsed from
    Source(Text),
}

pub trait HasTag {
    /// Get the associated metadata
    fn tag(&self) -> Tag;
}

/// A wrapper type that attaches a Span to a value
#[derive(new, Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Hash)]
pub struct Spanned<T> {
    pub span: Span,
    pub item: T,
}

impl<T> Spanned<T> {
    /// Allows mapping over a Spanned value
    pub fn map<U>(self, input: impl FnOnce(T) -> U) -> Spanned<U> {
        let span = self.span;

        let mapped = input(self.item);
        mapped.spanned(span)
    }
}

impl Spanned<String> {
    /// Iterates over the contained String
    pub fn items<'a, U>(
        items: impl Iterator<Item = &'a Spanned<String>>,
    ) -> impl Iterator<Item = &'a str> {
        items.map(|item| &item.item[..])
    }

    /// Borrows the contained String
    pub fn borrow_spanned(&self) -> Spanned<&str> {
        let span = self.span;
        self.item[..].spanned(span)
    }

    pub fn slice_spanned(&self, span: impl Into<Span>) -> Spanned<&str> {
        let span = span.into();
        let item = &self.item[span.start()..span.end()];
        item.spanned(span)
    }
}

pub trait SpannedItem: Sized {
    /// Converts a value into a Spanned value
    fn spanned(self, span: impl Into<Span>) -> Spanned<Self> {
        Spanned {
            item: self,
            span: span.into(),
        }
    }

    /// Converts a value into a Spanned value, using an unknown Span
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

    /// Shorthand to deref to the contained value
    fn deref(&self) -> &T {
        &self.item
    }
}

/// A wrapper type that attaches a Tag to a value
#[derive(new, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Hash)]
pub struct Tagged<T> {
    pub tag: Tag,
    pub item: T,
}

impl Tagged<String> {
    /// Allows borrowing the contained string slice as a spanned value
    pub fn borrow_spanned(&self) -> Spanned<&str> {
        let span = self.tag.span;
        self.item[..].spanned(span)
    }

    /// Allows borrowing the contained string slice as a tagged value
    pub fn borrow_tagged(&self) -> Tagged<&str> {
        self.item[..].tagged(self.tag.clone())
    }
}

impl<T> Tagged<Vec<T>> {
    /// Iterates over the contained value(s)
    pub fn items(&self) -> impl Iterator<Item = &T> {
        self.item.iter()
    }
}

impl<T> HasTag for Tagged<T> {
    /// Helper for getting the Tag from the Tagged value
    fn tag(&self) -> Tag {
        self.tag.clone()
    }
}

impl AsRef<Path> for Tagged<PathBuf> {
    /// Gets the reference to the contained Path
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

    /// Creates a new `Tag` from the current `Tag`
    pub fn tag(&self) -> Tag {
        self.tag.clone()
    }

    /// Retrieve the `Span` for the current `Tag`.
    pub fn span(&self) -> Span {
        self.tag.span
    }

    /// Returns the `AnchorLocation` of the `Tag` if there is one.
    pub fn anchor(&self) -> Option<AnchorLocation> {
        self.tag.anchor.clone()
    }

    /// Returns the underlying `AnchorLocation` variant type as a string.
    pub fn anchor_name(&self) -> Option<String> {
        match self.tag.anchor {
            Some(AnchorLocation::File(ref file)) => Some(file.clone()),
            Some(AnchorLocation::Url(ref url)) => Some(url.clone()),
            _ => None,
        }
    }

    /// Returns a reference to the current `Tag`'s item.
    pub fn item(&self) -> &T {
        &self.item
    }

    /// Returns a tuple of the `Tagged` item and `Tag`.
    pub fn into_parts(self) -> (T, Tag) {
        (self.item, self.tag)
    }
}

impl From<&Tag> for Tag {
    fn from(input: &Tag) -> Tag {
        input.clone()
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

/// The set of metadata that can be associated with a value
#[derive(
    Debug,
    Default,
    Clone,
    PartialEq,
    Eq,
    Ord,
    PartialOrd,
    Serialize,
    Deserialize,
    Hash,
    Getters,
    new,
)]
pub struct Tag {
    /// The original source for this value
    pub anchor: Option<AnchorLocation>,
    /// The span in the source text for the command that created this value
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
    /// Creates a default `Tag' with unknown `Span` position and no `AnchorLocation`
    pub fn default() -> Self {
        Tag {
            anchor: None,
            span: Span::unknown(),
        }
    }

    pub fn anchored(self, anchor: Option<AnchorLocation>) -> Tag {
        Tag {
            anchor,
            span: self.span,
        }
    }

    /// Creates a `Tag` from the given `Span` with no `AnchorLocation`
    pub fn unknown_anchor(span: Span) -> Tag {
        Tag { anchor: None, span }
    }

    /// Creates a `Tag` from the given `AnchorLocation` for a span with a length of 1.
    pub fn for_char(pos: usize, anchor: AnchorLocation) -> Tag {
        Tag {
            anchor: Some(anchor),
            span: Span::new(pos, pos + 1),
        }
    }

    /// Creates a `Tag` for the given `AnchorLocation` with unknown `Span` position.
    pub fn unknown_span(anchor: AnchorLocation) -> Tag {
        Tag {
            anchor: Some(anchor),
            span: Span::unknown(),
        }
    }

    /// Creates a `Tag` with no `AnchorLocation` and an unknown `Span` position.
    pub fn unknown() -> Tag {
        Tag {
            anchor: None,
            span: Span::unknown(),
        }
    }

    /// Returns the `AnchorLocation` of the current `Tag`
    pub fn anchor(&self) -> Option<AnchorLocation> {
        self.anchor.clone()
    }

    // Merges the current `Tag` with the given `Tag`.
    ///
    /// Both Tags must share the same `AnchorLocation`.
    // The resulting `Tag` will have a `Span` that starts from the current `Tag` and ends at `Span` of the given `Tag`.
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

    /// Merges the current `Tag` with the given optional `Tag`.
    ///
    /// Both `Tag`s must share the same `AnchorLocation`.
    /// The resulting `Tag` will have a `Span` that starts from the current `Tag` and ends at `Span` of the given `Tag`.
    /// Should the `None` variant be passed in, a new `Tag` with the same `Span` and `AnchorLocation` will be returned.
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

    pub fn string(&self, source: &str) -> String {
        self.span.slice(source).to_string()
    }

    pub fn tagged_slice<'a>(&self, source: &'a str) -> Tagged<&'a str> {
        self.span.slice(source).tagged(self)
    }

    pub fn tagged_string(&self, source: &str) -> Tagged<String> {
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

/// A `Span` is metadata which indicates the start and end positions.
///
/// `Span`s are combined with `AnchorLocation`s to form another type of metadata, a `Tag`.
/// A `Span`'s end position must be greater than or equal to its start position.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize, Hash,
)]
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
        input.unwrap_or_else(|| Span::new(0, 0))
    }
}

impl From<Span> for std::ops::Range<usize> {
    fn from(input: Span) -> std::ops::Range<usize> {
        std::ops::Range {
            start: input.start,
            end: input.end,
        }
    }
}

impl Span {
    /// Creates a default new `Span` that has 0 start and 0 end.
    pub fn default() -> Self {
        Span::unknown()
    }

    /// Creates a new `Span` that has 0 start and 0 end.
    pub fn unknown() -> Span {
        Span::new(0, 0)
    }

    pub fn from_list(list: &[impl HasSpan]) -> Span {
        let mut iterator = list.iter();

        match iterator.next() {
            None => Span::new(0, 0),
            Some(first) => {
                let last = iterator.last().unwrap_or(first);

                Span::new(first.span().start, last.span().end)
            }
        }
    }

    /// Creates a new `Span` from start and end inputs. The end parameter must be greater than or equal to the start parameter.
    pub fn new(start: usize, end: usize) -> Span {
        assert!(
            end >= start,
            "Can't create a Span whose end < start, start={}, end={}",
            start,
            end
        );

        Span { start, end }
    }

    /// Creates a `Span` with a length of 1 from the given position.
    ///
    /// # Example
    ///
    /// ```
    /// let char_span = Span::for_char(5);
    ///
    /// assert_eq!(char_span.start(), 5);
    /// assert_eq!(char_span.end(), 6);
    /// ```
    pub fn for_char(pos: usize) -> Span {
        Span {
            start: pos,
            end: pos + 1,
        }
    }

    /// Returns a bool indicating if the given position falls inside the current `Span`.
    ///
    /// # Example
    ///
    /// ```
    /// let span = Span::new(2, 8);
    ///
    /// assert_eq!(span.contains(5), true);
    /// assert_eq!(span.contains(8), false);
    /// assert_eq!(span.contains(100), false);
    /// ```
    pub fn contains(&self, pos: usize) -> bool {
        self.start <= pos && pos < self.end
    }

    /// Returns a new Span by merging an earlier Span with the current Span.
    ///
    /// The resulting Span will have the same start position as the given Span and same end as the current Span.
    ///
    /// # Example
    ///
    /// ```
    /// let original_span = Span::new(4, 6);
    /// let earlier_span = Span::new(1, 3);
    /// let merged_span = origin_span.since(earlier_span);
    ///
    /// assert_eq!(merged_span.start(), 1);
    /// assert_eq!(merged_span.end(), 6);
    /// ```
    pub fn since(&self, other: impl Into<Span>) -> Span {
        let other = other.into();

        Span::new(other.start, self.end)
    }

    /// Returns a new Span by merging a later Span with the current Span.
    ///
    /// The resulting Span will have the same start position as the current Span and same end as the given Span.
    ///
    /// # Example
    ///
    /// ```
    /// let original_span = Span::new(4, 6);
    /// let later_span = Span::new(9, 11);
    /// let merged_span = origin_span.until(later_span);
    ///
    /// assert_eq!(merged_span.start(), 4);
    /// assert_eq!(merged_span.end(), 11);
    /// ```
    pub fn until(&self, other: impl Into<Span>) -> Span {
        let other = other.into();

        Span::new(self.start, other.end)
    }

    /// Returns a new Span by merging a later Span with the current Span.
    ///
    /// If the given Span is of the None variant,
    /// A Span with the same values as the current Span is returned.
    pub fn until_option(&self, other: Option<impl Into<Span>>) -> Span {
        match other {
            Some(other) => {
                let other = other.into();

                Span::new(self.start, other.end)
            }
            None => *self,
        }
    }

    pub fn string(&self, source: &str) -> String {
        self.slice(source).to_string()
    }

    pub fn spanned_slice<'a>(&self, source: &'a str) -> Spanned<&'a str> {
        self.slice(source).spanned(*self)
    }

    pub fn spanned_string(&self, source: &str) -> Spanned<String> {
        self.slice(source).to_string().spanned(*self)
    }

    /// Returns the start position of the current Span.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the end position of the current Span.
    pub fn end(&self) -> usize {
        self.end
    }

    /// Returns a bool if the current Span indicates an "unknown"  position.
    ///
    /// # Example
    ///
    /// ```
    /// let unknown_span = Span::unknown();
    /// let known_span = Span::new(4, 6);
    ///
    /// assert_eq!(unknown_span.is_unknown(), true);
    /// assert_eq!(known_span.is_unknown(), false);
    /// ```
    pub fn is_unknown(&self) -> bool {
        self.start == 0 && self.end == 0
    }

    /// Returns a bool if the current Span does not cover.
    ///
    /// # Example
    ///
    /// ```
    /// //  make clean
    /// //  ----
    /// //  (0,4)
    /// //
    /// //       ^(5,5)
    ///
    /// let make_span = Span::new(0,4);
    /// let clean_span = Span::new(5,5);
    ///
    /// assert_eq!(make_span.is_closed(), false);
    /// assert_eq!(clean_span.is_closed(), true);
    /// ```
    pub fn is_closed(&self) -> bool {
        self.start == self.end
    }

    /// Returns a slice of the input that covers the start and end of the current Span.
    pub fn slice<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }
}

impl PartialOrd<usize> for Span {
    fn partial_cmp(&self, other: &usize) -> Option<Ordering> {
        (self.end - self.start).partial_cmp(other)
    }
}

impl PartialEq<usize> for Span {
    fn eq(&self, other: &usize) -> bool {
        (self.end - self.start) == *other
    }
}

pub trait IntoSpanned {
    type Output: HasFallibleSpan;

    fn into_spanned(self, span: impl Into<Span>) -> Self::Output;
}

impl<T: HasFallibleSpan> IntoSpanned for T {
    type Output = T;
    fn into_spanned(self, _span: impl Into<Span>) -> Self::Output {
        self
    }
}

pub trait HasSpan {
    fn span(&self) -> Span;
}

impl<T, E> HasSpan for Result<T, E>
where
    T: HasSpan,
{
    fn span(&self) -> Span {
        match self {
            Result::Ok(val) => val.span(),
            Result::Err(_) => Span::unknown(),
        }
    }
}

impl<T> HasSpan for Spanned<T> {
    fn span(&self) -> Span {
        self.span
    }
}

pub trait HasFallibleSpan {
    fn maybe_span(&self) -> Option<Span>;
}

impl HasFallibleSpan for bool {
    fn maybe_span(&self) -> Option<Span> {
        None
    }
}

impl HasFallibleSpan for () {
    fn maybe_span(&self) -> Option<Span> {
        None
    }
}

impl<T> HasFallibleSpan for T
where
    T: HasSpan,
{
    fn maybe_span(&self) -> Option<Span> {
        Some(HasSpan::span(self))
    }
}

impl PrettyDebugWithSource for Option<Span> {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self {
            None => DbgDocBldr::description("no span"),
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
        DbgDocBldr::typed(
            "span",
            DbgDocBldr::keyword("for")
                + DbgDocBldr::space()
                + DbgDocBldr::description(format!("{:?}", self.slice(source))),
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
            None => DbgDocBldr::description("nothing"),
            Some(v) => v.pretty_debug(v.span.slice(source)),
        }
    }
}

impl<T> HasFallibleSpan for Option<Spanned<T>> {
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
            None => DbgDocBldr::description("nothing"),
            Some(d) => d.pretty_debug(source),
        }
    }
}

impl<T> HasFallibleSpan for Option<Tagged<T>> {
    fn maybe_span(&self) -> Option<Span> {
        match self {
            None => None,
            Some(value) => Some(value.tag.span),
        }
    }
}

impl<T> HasSpan for Tagged<T> {
    fn span(&self) -> Span {
        self.tag.span
    }
}
