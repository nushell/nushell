use std::{
    ops::{Deref, DerefMut},
    slice::SliceIndex,
};

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
pub struct PointedSpanArray<'a, Idx> {
    inner: &'a [Span],
    idx: Idx,
}

impl<'a> PointedSpanArray<'a, NestedUsize> {
    #[inline]
    #[must_use]
    pub fn new(value: &'a [Span], idx: usize) -> Option<Self> {
        Self::new_inner(value, NestedUsize(idx))
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
impl<'a, 'b, Idx: 'b> PointedSpanArray<'a, Idx>
where
    Idx: Deref<Target = usize>,
{
    #[inline]
    #[must_use]
    pub fn new_inner(value: &'a [Span], idx: Idx) -> Option<Self> {
        // check valid index, otherwise return none
        _ = value.get(*idx)?;
        Some(PointedSpanArray { inner: value, idx })
    }

    /// Get the span at the current index
    pub fn current(&self) -> Span {
        // debug_assert!(self.inner.len() > *self.idx, "expect spans > 0");
        // Safe, since the index is checked on construction
        self.inner[*self.idx]
    }

    pub fn get_slice(&self) -> &'a [Span] {
        self.inner
    }
    pub fn get_idx(&self) -> usize {
        *self.idx
    }

    /// Get the spans starting at the current index
    pub fn tail_inclusive(&self) -> SpanArray<'a> {
        // Safe, since the index is checked on construction
        SpanArray {
            inner: &self.inner[*self.idx..],
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
        self.get_at(*self.idx + 1)
    }
}

impl<'a, 'b, Idx: 'b> PointedSpanArray<'a, Idx>
where
    Idx: Deref<Target = usize>,
    Idx: DerefMut<Target = usize>,
{
    #[inline]
    #[must_use]
    pub fn sub_span<I>(&mut self, range: I) -> Option<PointedSpanArray<'a, NestedRef<Idx>>>
    where
        I: SliceIndex<[Span], Output = [Span]>,
    {
        PointedSpanArray::new_inner(self.inner.get(range)?, NestedRef(&mut self.idx))
    }
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
}

pub struct NestedUsize(usize);
impl Deref for NestedUsize {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for NestedUsize {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Allows for any number of layers deep references
pub struct NestedRef<'a, T>(&'a mut T);

impl<'a, T> Deref for NestedRef<'a, T>
where
    T: Deref,
{
    type Target = T::Target;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a, T> DerefMut for NestedRef<'a, T>
where
    T: DerefMut,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}
