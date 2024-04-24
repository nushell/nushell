use std::ops::Deref;

use miette::SourceSpan;
use serde::{Deserialize, Serialize};

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
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl From<Span> for SourceSpan {
    fn from(s: Span) -> Self {
        Self::new(s.start.into(), s.end - s.start)
    }
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        debug_assert!(
            end >= start,
            "Can't create a Span whose end < start, start={start}, end={end}"
        );

        Span { start, end }
    }

    pub const fn unknown() -> Span {
        Span { start: 0, end: 0 }
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub const fn test_data() -> Span {
        Self::unknown()
    }

    pub fn offset(&self, offset: usize) -> Span {
        Span::new(self.start - offset, self.end - offset)
    }

    pub fn contains(&self, pos: usize) -> bool {
        pos >= self.start && pos < self.end
    }

    pub fn contains_span(&self, span: Span) -> bool {
        span.start >= self.start && span.end <= self.end
    }

    /// Point to the space just past this span, useful for missing
    /// values
    pub fn past(&self) -> Span {
        Span {
            start: self.end,
            end: self.end,
        }
    }
}

/// Used when you have a slice of spans of at least size 1
pub fn span(spans: &[Span]) -> Span {
    let length = spans.len();

    //TODO debug_assert!(length > 0, "expect spans > 0");
    if length == 0 {
        Span::unknown()
    } else if length == 1 {
        spans[0]
    } else {
        let end = spans
            .iter()
            .map(|s| s.end)
            .max()
            .expect("Must be an end. Length > 0");
        Span::new(spans[0].start, end)
    }
}

/// An extension trait for `Result`, which adds a span to the error type.
pub trait ErrSpan {
    type Result;

    /// Add the given span to the error type `E`, turning it into a `Spanned<E>`.
    ///
    /// Some auto-conversion methods to `ShellError` from other error types are available on spanned
    /// errors, to give users better information about where an error came from. For example, it is
    /// preferred when working with `std::io::Error`:
    ///
    /// ```no_run
    /// use nu_protocol::{ErrSpan, ShellError, Span};
    /// use std::io::Read;
    ///
    /// fn read_from(mut reader: impl Read, span: Span) -> Result<Vec<u8>, ShellError> {
    ///     let mut vec = vec![];
    ///     reader.read_to_end(&mut vec).err_span(span)?;
    ///     Ok(vec)
    /// }
    /// ```
    fn err_span(self, span: Span) -> Self::Result;
}

impl<T, E> ErrSpan for Result<T, E> {
    type Result = Result<T, Spanned<E>>;

    fn err_span(self, span: Span) -> Self::Result {
        self.map_err(|err| err.into_spanned(span))
    }
}
