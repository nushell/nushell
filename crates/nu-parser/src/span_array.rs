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
        value
            .get(*idx)
            .map(|_| PointedSpanArray { inner: value, idx })
    }
    pub fn get(self) -> Span {
        self.inner[*self.idx]
    }
    // pub fn get_last(self) -> Span {
    //     self.inner[self.inner.len() - 1]
    // }
    #[inline]
    #[must_use]
    pub fn get_at(self, index: usize) -> Option<Span> {
        self.inner.get(index).map(|&x| x)
    }
    #[inline]
    #[must_use]
    pub fn try_advance(self) -> bool {
        if *self.idx + 1 < self.inner.len() {
            *self.idx += 1;
            true
        } else {
            false
        }
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
