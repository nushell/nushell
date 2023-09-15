use std::slice::SliceIndex;

// use miette::SourceSpan;
// use serde::{Deserialize, Serialize};

use nu_protocol::Span;

/// Arrays of spans that are guaranteed to be non-empty by construction
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpanArray<'a> {
    inner: &'a [Span],
}

impl<'a> TryFrom<&'a [Span]> for SpanArray<'a> {
    type Error = &'static str;

    fn try_from(value: &'a [Span]) -> Result<Self, Self::Error> {
        Self::new(value).ok_or("Got empty array")
    }
}

impl<'a> From<SpanArray<'a>> for &'a [Span] {
    fn from(value: SpanArray<'a>) -> Self {
        value.inner
    }
}

impl<'a> SpanArray<'a> {
    #[inline]
    #[must_use]
    pub fn new(value: &'a [Span]) -> Option<Self> {
        if value.is_empty() {
            None
        } else {
            Some(SpanArray { inner: value })
        }
    }
    #[inline]
    #[must_use]
    pub fn get(self, index: usize) -> Option<Span> {
        self.inner.get(index).map(|&x| x)
    }

    #[inline]
    #[must_use]
    pub fn slice<I>(self, index: I) -> Option<Self>
    where
        I: SliceIndex<[Span], Output = [Span]>,
    {
        self.inner.get(index).and_then(|x| Self::new(x))
    }
}

// This is almost an iterator, can it actually be one?
/// An array of spans and an index into that array
#[derive(Debug)]
pub struct PointedSpanArray<'a, 'b> {
    inner: &'a [Span],
    idx: &'b mut usize,
}

impl<'a, 'b> PointedSpanArray<'a, 'b> {
    #[inline]
    #[must_use]
    pub fn new(value: &'a [Span], idx: &'b mut usize) -> Option<Self> {
        // check valid index, otherwise return none
        _ = value.get(*idx)?;
        Some(PointedSpanArray { inner: value, idx })
    }

    #[inline]
    #[must_use]
    pub fn new_from_range<I>(span: &'a [Span], range: I, idx: &'b mut usize) -> Option<Self>
    where
        I: SliceIndex<[Span], Output = [Span]>,
    {
        // Check valid index, otherwise return None
        let new_span = span.get(range)?;
        Self::new(new_span, idx)
    }

    /// Get the span at the current index
    pub fn current(&self) -> Span {
        // debug_assert!(self.inner.len() > *self.idx, "expect spans > 0");
        // Safe, since the index is checked on construction
        self.inner[*self.idx]
    }

    /// Get the spans starting at the current index
    pub fn tail_inclusive(&self) -> SpanArray<'a> {
        // Safe, since the index is checked on construction
        SpanArray {
            inner: &self.inner[*self.idx..],
        }
    }

    // pub fn get_last(self) -> Span {
    //     self.inner[self.inner.len() - 1]
    // }

    /// Get the value at an index
    #[inline]
    #[must_use]
    pub fn get_at(&self, index: usize) -> Option<Span> {
        Some(*self.inner.get(index)?)
    }

    // /// Get the next n spans after the index
    // #[inline]
    // #[must_use]
    // pub fn peek_n(self, number: usize) -> Option<Span> {
    //     self.slice_arr(*self.idx..*self.idx + n)
    // }

    // TODO: Maybe return next value here
    #[inline]
    #[must_use]
    pub fn try_advance(&mut self) -> bool {
        if *self.idx + 1 < self.inner.len() {
            *self.idx += 1;
            true
        } else {
            false
        }
    }

    pub fn jump_to_end(&mut self) {
        *self.idx = self.inner.len() - 1;
    }

    pub fn is_at_end(&self) -> bool {
        *self.idx == self.inner.len() - 1
    }

    #[inline]
    #[must_use]
    pub fn peek_next(&self) -> Option<Span> {
        self.get_at(*self.idx + 1)
    }

    // #[inline]
    // #[must_use]
    // pub fn slice<I>(self, index: I) -> Option<Self>
    // where
    //     I: SliceIndex<[Span], Output = [Span]>,
    // {
    //     self.inner.get(index).and_then(|x| Self::new(x))
    // }
}
