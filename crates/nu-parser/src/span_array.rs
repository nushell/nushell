use std::{ops::RangeBounds, slice::SliceIndex};

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
        self.inner.get(index).copied()
    }

    #[inline]
    #[must_use]
    pub fn slice<I>(self, index: I) -> Option<Self>
    where
        I: SliceIndex<[Span], Output = [Span]>,
    {
        self.inner.get(index).and_then(Self::new)
    }
}

// This is almost an iterator, can it actually be one?
/// An array of spans and an index into that array
#[derive(Debug)]
pub struct PointedSpanArray<'a> {
    inner: &'a [Span],
    idx: usize,
}

impl<'a> PointedSpanArray<'a> {
    #[inline]
    #[must_use]
    pub fn new(value: &'a [Span], idx: usize) -> Option<Self> {
        Self::new_inner(value, idx)
    }

    #[inline]
    #[must_use]
    pub fn new_from_range<I>(spans: &'a [Span], range: I, idx: usize) -> Option<Self>
    where
        I: SliceIndex<[Span], Output = [Span]>,
    {
        // Check valid index, otherwise return None
        Self::new(spans.get(range)?, idx)
    }
}
impl<'a> PointedSpanArray<'a> {
    #[inline]
    #[must_use]
    pub fn new_inner(value: &'a [Span], idx: usize) -> Option<Self> {
        // check valid index, otherwise return none
        _ = value.get(idx)?;
        Some(PointedSpanArray { inner: value, idx })
    }

    /// Get the span at the current index
    pub fn current(&self) -> Span {
        // debug_assert!(self.inner.len() > self.idx, "expect spans > 0");
        // Safe, since the index is checked on construction
        self.inner[self.idx]
    }

    pub fn get_slice(&self) -> &'a [Span] {
        self.inner
    }
    pub fn get_idx(&self) -> usize {
        self.idx
    }

    /// Get the spans starting at the current index
    pub fn tail_inclusive(&self) -> SpanArray<'a> {
        // Safe, since the index is checked on construction
        SpanArray {
            inner: &self.inner[self.idx..],
        }
    }

    /// Get the value at an index
    #[inline]
    #[must_use]
    pub fn get_at(&self, index: usize) -> Option<Span> {
        Some(*self.inner.get(index)?)
    }

    #[inline]
    #[must_use]
    pub fn peek_next(&self) -> Option<Span> {
        self.get_at(self.idx + 1)
    }
}

impl<'a> PointedSpanArray<'a> {
    // /// Make a new span array of a prefix, sharing the index with the original
    // #[inline]
    // #[must_use]
    // pub fn prefix_span(&mut self, end: usize) -> Option<self> {
    //     PointedSpanArray::new_inner(self.inner.get(..end)?, NestedRef(&mut self.idx))
    // }

    // TODO: Maybe return next value here
    #[inline]
    #[must_use]
    pub fn try_advance(&mut self) -> bool {
        if self.idx + 1 < self.inner.len() {
            self.idx += 1;
            true
        } else {
            false
        }
    }

    pub fn jump_to_end(&mut self) {
        self.idx = self.inner.len() - 1;
    }
}

impl<'a> PointedSpanArray<'a> {
    #[inline]
    #[must_use]
    pub fn with_sub_span<I, F, T>(&mut self, range: I, callback: F) -> Option<T>
    where
        I: SliceIndex<[Span], Output = [Span]>,
        I: RangeBounds<usize>,
        F: FnOnce(&mut PointedSpanArray<'a>) -> T,
    {
        let start_idx = match range.start_bound() {
            std::ops::Bound::Included(&n) => n,
            std::ops::Bound::Excluded(&n) => n + 1,
            std::ops::Bound::Unbounded => 0,
        };
        let new_idx = self.idx - start_idx;
        let mut sub_span = Self::new_from_range(self.inner, range, new_idx)?;
        let result = callback(&mut sub_span);
        self.idx = start_idx + sub_span.idx;
        Some(result)
    }
}
