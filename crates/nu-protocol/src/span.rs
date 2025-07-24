//! [`Span`] to point to sections of source code and the [`Spanned`] wrapper type
use crate::{IntoValue, SpanId, Value, record};
use miette::SourceSpan;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

pub trait GetSpan {
    fn get_span(&self, span_id: SpanId) -> Span;
}

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

impl<T, E> Spanned<Result<T, E>> {
    /// Move the `Result` to the outside, resulting in a spanned `Ok` or unspanned `Err`.
    pub fn transpose(self) -> Result<Spanned<T>, E> {
        match self {
            Spanned {
                item: Ok(item),
                span,
            } => Ok(Spanned { item, span }),
            Spanned {
                item: Err(err),
                span: _,
            } => Err(err),
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

    /// Span for testing purposes.
    ///
    /// The provided span does not point into any known source but is unequal to [`Span::unknown()`].
    ///
    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors
    pub const fn test_data() -> Self {
        Self {
            start: usize::MAX / 2,
            end: usize::MAX / 2,
        }
    }

    pub fn offset(&self, offset: usize) -> Self {
        Self::new(self.start - offset, self.end - offset)
    }

    /// Return length of the slice.
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Indicate if slice has length 0.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Return another span fully inside the [`Span`].
    ///
    /// `start` and `end` are relative to `self.start`, and must lie within the `Span`.
    /// In other words, both `start` and `end` must be `<= self.len()`.
    pub fn subspan(&self, offset_start: usize, offset_end: usize) -> Option<Self> {
        let len = self.len();

        if offset_start > len || offset_end > len || offset_start > offset_end {
            None
        } else {
            Some(Self::new(
                self.start + offset_start,
                self.start + offset_end,
            ))
        }
    }

    /// Return two spans that split the ['Span'] at the given position.
    pub fn split_at(&self, offset: usize) -> Option<(Self, Self)> {
        if offset < self.len() {
            Some((
                Self::new(self.start, self.start + offset),
                Self::new(self.start + offset, self.end),
            ))
        } else {
            None
        }
    }

    pub fn contains(&self, pos: usize) -> bool {
        self.start <= pos && pos < self.end
    }

    pub fn contains_span(&self, span: Self) -> bool {
        self.start <= span.start && span.end <= self.end && span.end != 0
    }

    /// Point to the space just past this span, useful for missing values
    pub fn past(&self) -> Self {
        Self {
            start: self.end,
            end: self.end,
        }
    }

    /// Converts row and column in a String to a Span, assuming bytes (1-based rows)
    pub fn from_row_column(row: usize, col: usize, contents: &str) -> Span {
        let mut cur_row = 1;
        let mut cur_col = 1;

        for (offset, curr_byte) in contents.bytes().enumerate() {
            if curr_byte == b'\n' {
                cur_row += 1;
                cur_col = 1;
            } else if cur_row >= row && cur_col >= col {
                return Span::new(offset, offset);
            } else {
                cur_col += 1;
            }
        }

        Self {
            start: contents.len(),
            end: contents.len(),
        }
    }

    /// Returns the minimal [`Span`] that encompasses both of the given spans.
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

    /// Returns the minimal [`Span`] that encompasses both of the given spans.
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
    /// The spans are assumed to be in order, that is, all consecutive spans must satisfy:
    /// - `spans[i].start <= spans[i + 1].start`
    /// - `spans[i].end <= spans[i + 1].end`
    ///
    /// (Two consecutive spans can overlap as long as the above is true.)
    ///
    /// Use [`Span::merge_many`] if the spans are not known to be in order.
    pub fn concat(spans: &[Self]) -> Self {
        // TODO: enable assert below
        // debug_assert!(!spans.is_empty());
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

impl IntoValue for Span {
    fn into_value(self, span: Span) -> Value {
        let record = record! {
            "start" => Value::int(self.start as i64, self),
            "end" => Value::int(self.end as i64, self),
        };
        record.into_value(span)
    }
}

impl From<Span> for SourceSpan {
    fn from(s: Span) -> Self {
        Self::new(s.start.into(), s.end - s.start)
    }
}

/// An extension trait for [`Result`], which adds a span to the error type.
///
/// This trait might be removed later, since the old [`Spanned<std::io::Error>`] to
/// [`ShellError`](crate::ShellError) conversion was replaced by
/// [`IoError`](crate::shell_error::io::IoError).
pub trait ErrSpan {
    type Result;

    /// Adds the given span to the error type, turning it into a [`Spanned<E>`].
    fn err_span(self, span: Span) -> Self::Result;
}

impl<T, E> ErrSpan for Result<T, E> {
    type Result = Result<T, Spanned<E>>;

    fn err_span(self, span: Span) -> Self::Result {
        self.map_err(|err| err.into_spanned(span))
    }
}
