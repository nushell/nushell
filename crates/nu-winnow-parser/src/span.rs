//! Source positions.
//!
//! Every AST node carries a [`Span`]: a half-open byte range into the original
//! source text. Byte offsets are stable, cheap, and exactly what a formatter
//! needs to splice text back together; use [`LineIndex`] to convert them to
//! line/column pairs for display.

use std::fmt;
use std::ops::Range;

/// A half-open byte range `[start, end)` into the source text.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Span {
    /// Byte offset of the first byte.
    pub start: usize,
    /// Byte offset one past the last byte.
    pub end: usize,
}

impl Span {
    /// Create a span from byte offsets.
    #[inline]
    pub const fn new(start: usize, end: usize) -> Self {
        debug_assert!(start <= end);
        Self { start, end }
    }

    /// An empty span at `offset`.
    #[inline]
    pub const fn point(offset: usize) -> Self {
        Self { start: offset, end: offset }
    }

    /// Length in bytes.
    #[inline]
    pub const fn len(&self) -> usize {
        self.end - self.start
    }

    /// `true` if the span covers no bytes.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// The smallest span covering both `self` and `other`.
    #[inline]
    pub fn merge(self, other: Span) -> Span {
        Span::new(self.start.min(other.start), self.end.max(other.end))
    }

    /// The empty span immediately after this one.
    #[inline]
    pub const fn past(&self) -> Span {
        Span::point(self.end)
    }

    /// `true` if `offset` lies inside the span (or at its start for empty spans).
    #[inline]
    pub const fn contains(&self, offset: usize) -> bool {
        self.start <= offset && (offset < self.end || (self.start == self.end && offset == self.end))
    }

    /// The text this span refers to.
    ///
    /// # Panics
    /// Panics if the span is out of bounds or does not fall on char boundaries
    /// of `source`. Spans produced by this crate always satisfy both.
    #[inline]
    pub fn slice<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }

    /// Convert to a standard range.
    #[inline]
    pub const fn range(&self) -> Range<usize> {
        self.start..self.end
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

impl From<Range<usize>> for Span {
    fn from(r: Range<usize>) -> Self {
        Span::new(r.start, r.end)
    }
}

impl From<Span> for Range<usize> {
    fn from(s: Span) -> Self {
        s.range()
    }
}

/// A value together with the span it was parsed from.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Spanned<T> {
    /// The value.
    pub item: T,
    /// Where it came from.
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Pair a value with a span.
    pub const fn new(item: T, span: Span) -> Self {
        Self { item, span }
    }

    /// Map the inner value, keeping the span.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned { item: f(self.item), span: self.span }
    }
}

/// A 1-based line/column position.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct LineCol {
    /// 1-based line number.
    pub line: usize,
    /// 1-based column, counted in characters (not bytes).
    pub column: usize,
}

impl fmt::Display for LineCol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Precomputed line starts for converting byte offsets to [`LineCol`].
#[derive(Clone, Debug)]
pub struct LineIndex {
    line_starts: Vec<usize>,
    len: usize,
}

impl LineIndex {
    /// Build an index for `source`.
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(source.match_indices('\n').map(|(i, _)| i + 1));
        Self { line_starts, len: source.len() }
    }

    /// Number of lines (a trailing newline does not start a new line).
    pub fn line_count(&self) -> usize {
        let n = self.line_starts.len();
        if n > 1 && self.line_starts[n - 1] == self.len { n - 1 } else { n }
    }

    /// The 0-based line index containing `offset`.
    pub fn line_of(&self, offset: usize) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        }
    }

    /// Byte range of the given 0-based line, excluding the line terminator.
    pub fn line_range(&self, line: usize, source: &str) -> Range<usize> {
        let start = self.line_starts[line];
        let end = self.line_starts.get(line + 1).map_or(source.len(), |e| e - 1);
        let end = end.max(start);
        let end = if source.as_bytes().get(end.wrapping_sub(1)) == Some(&b'\r') && end > start { end - 1 } else { end };
        start..end
    }

    /// Convert a byte offset to a 1-based line/column.
    pub fn line_col(&self, offset: usize, source: &str) -> LineCol {
        let offset = offset.min(source.len());
        let line = self.line_of(offset);
        let start = self.line_starts[line];
        let column = source[start..offset].chars().count() + 1;
        LineCol { line: line + 1, column }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_index_positions() {
        let src = "ab\ncd\r\nef";
        let idx = LineIndex::new(src);
        assert_eq!(idx.line_col(0, src), LineCol { line: 1, column: 1 });
        assert_eq!(idx.line_col(2, src), LineCol { line: 1, column: 3 });
        assert_eq!(idx.line_col(3, src), LineCol { line: 2, column: 1 });
        assert_eq!(idx.line_col(7, src), LineCol { line: 3, column: 1 });
        assert_eq!(idx.line_col(9, src), LineCol { line: 3, column: 3 });
        assert_eq!(idx.line_range(1, src), 3..5);
        assert_eq!(idx.line_count(), 3);
        assert_eq!(LineIndex::new("a\n").line_count(), 1);
    }

    #[test]
    fn span_ops() {
        let s = Span::new(2, 5);
        assert_eq!(s.len(), 3);
        assert_eq!(s.merge(Span::new(0, 3)), Span::new(0, 5));
        assert_eq!(s.past(), Span::point(5));
        assert!(s.contains(2));
        assert!(!s.contains(5));
        assert_eq!(s.slice("hello!"), "llo");
    }
}
