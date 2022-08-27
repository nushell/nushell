use miette::SourceSpan;
use serde::{Deserialize, Serialize};

/// A spanned area of interest, generic over what kind of thing is of interest
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Spanned<T>
where
    T: Clone + std::fmt::Debug,
{
    pub item: T,
    pub span: Span,
}

/// Spans are a global offset across all seen files, which are cached in the engine's state. The start and
/// end offset together make the inclusive start/exclusive end pair for where to underline to highlight
/// a given point of interest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl From<Span> for SourceSpan {
    fn from(s: Span) -> Self {
        Self::new(s.start.into(), (s.end - s.start).into())
    }
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }

    pub fn unknown() -> Self {
        Self::new(0, 0)
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_data() -> Span {
        Span { start: 0, end: 0 }
    }

    pub fn offset(&self, offset: usize) -> Span {
        Span {
            start: self.start - offset,
            end: self.end - offset,
        }
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

    if length == 0 {
        // TODO: do this for now, but we might also want to protect against this case
        Span { start: 0, end: 0 }
    } else if length == 1 {
        spans[0]
    } else {
        Span {
            start: spans[0].start,
            end: spans[length - 1].end,
        }
    }
}
