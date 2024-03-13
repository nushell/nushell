use miette::SourceSpan;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

/// A spanned area of interest, generic over what kind of thing is of interest
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Spanned<T> {
    pub item: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Map to a spanned reference of the inner type, i.e. `Spanned<T> -> Spanned<&T>`.
    pub fn as_ref(&self) -> Spanned<&T> {
        Spanned {
            item: &self.item,
            span: self.span,
        }
    }

    /// Map to a mutable reference of the inner type, i.e. `Spanned<T> -> Spanned<&mut T>`.
    pub fn as_mut(&mut self) -> Spanned<&mut T> {
        Spanned {
            item: &mut self.item,
            span: self.span,
        }
    }

    /// Map to the result of [`.deref()`](std::ops::Deref::deref) on the inner type.
    ///
    /// This can be used for example to turn `Spanned<Vec<T>>` into `Spanned<&[T]>`.
    pub fn as_deref(&self) -> Spanned<&<T as Deref>::Target>
    where
        T: Deref,
    {
        Spanned {
            item: self.item.deref(),
            span: self.span,
        }
    }

    /// Map the spanned item with a function.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            item: f(self.item),
            span: self.span,
        }
    }
}

/// Helper trait to create [`Spanned`] more ergonomically.
pub trait IntoSpanned: Sized {
    /// Wrap items together with a span into [`Spanned`].
    ///
    /// # Example
    ///
    /// ```
    /// # use nu_protocol::{Span, IntoSpanned};
    /// # let span = Span::test_data();
    /// let spanned = "Hello, world!".into_spanned(span);
    /// assert_eq!("Hello, world!", spanned.item);
    /// assert_eq!(span, spanned.span);
    /// ```
    fn into_spanned(self, span: Span) -> Spanned<Self>;
}

impl<T> IntoSpanned for T {
    fn into_spanned(self, span: Span) -> Spanned<Self> {
        Spanned { item: self, span }
    }
}

/// Spans are a global offset across all seen files, which are cached in the engine's state. The start and
/// end offset together make the inclusive start/exclusive end pair for where to underline to highlight
/// a given point of interest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        debug_assert!(
            end >= start,
            "Can't create a Span whose end < start, start={start}, end={end}"
        );

        Self { start, end }
    }

    pub const fn unknown() -> Self {
        Self { start: 0, end: 0 }
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub const fn test_data() -> Self {
        Self::unknown()
    }

    pub fn offset(&self, offset: usize) -> Self {
        Self::new(self.start - offset, self.end - offset)
    }

    pub fn contains(&self, pos: usize) -> bool {
        self.start <= pos && pos < self.end
    }

    pub fn contains_span(&self, span: Self) -> bool {
        self.start <= span.start && span.end <= self.end
    }

    /// Point to the space just past this span, useful for missing values
    pub fn past(&self) -> Self {
        Self {
            start: self.end,
            end: self.end,
        }
    }

    /// Returns the minimal [`Span`] that encompases both of the given spans.
    ///
    /// The two `Spans` can overlap in the middle,
    /// but must otherwise be in order by satisfying:
    /// - `self.start <= after.start`
    /// - `self.end <= after.end`
    ///
    /// If this is not guaranteed to be the case, use [`Span::merge`] instead.
    pub fn append(self, after: Self) -> Self {
        debug_assert!(
            self.start <= after.start && self.end <= after.end,
            "Can't merge two Spans that are not in order"
        );
        Self {
            start: self.start,
            end: after.end,
        }
    }

    /// Returns the minimal [`Span`] that encompases both of the given spans.
    ///
    /// The spans need not be in order or have any relationship.
    ///
    /// [`Span::append`] is slightly more efficient if the spans are known to be in order.
    pub fn merge(self, other: Self) -> Self {
        Self {
            start: usize::min(self.start, other.start),
            end: usize::max(self.end, other.end),
        }
    }

    /// Returns the minimal [`Span`] that encompasses all of the spans in the given slice.
    ///
    /// The spans are assummed to be in order, that is, all consecutive spans must satisfy:
    /// - `spans[i].start <= spans[i + 1].start`
    /// - `spans[i].end <= spans[i + 1].end`
    ///
    /// (Two consecutive spans can overlap as long as the above is true.)
    ///
    /// Use [`Span::from_unordered`] if the spans are not known to be in order.
    pub fn concat(spans: &[Self]) -> Self {
        debug_assert!(!spans.is_empty());
        debug_assert!(spans.windows(2).all(|spans| {
            let &[a, b] = spans else {
                return false;
            };
            a.start <= b.start && a.end <= b.end
        }));
        Self {
            start: spans.first().map(|s| s.start).unwrap_or(0),
            end: spans.last().map(|s| s.end).unwrap_or(0),
        }
    }

    /// Returns the minimal [`Span`] that encompasses all of the spans in the given iterator.
    ///
    /// The spans need not be in order or have any relationship.
    ///
    /// [`Span::concat`] is more efficient if the spans are known to be in order.
    pub fn merge_many(spans: impl IntoIterator<Item = Self>) -> Self {
        spans
            .into_iter()
            .reduce(Self::merge)
            .unwrap_or(Self::unknown())
    }
}

impl From<Span> for SourceSpan {
    fn from(s: Span) -> Self {
        Self::new(s.start.into(), s.end - s.start)
    }
}
