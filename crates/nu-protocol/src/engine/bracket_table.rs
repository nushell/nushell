use crate::Span;

/// Where the brackets of one source file close.
///
/// nu-parser's lexer builds one of these per file in a [`StateWorkingSet`](super::StateWorkingSet)
/// and uses it to jump from an opening `(`, `[` or `{` straight past its closer instead of scanning
/// the bytes in between. An entry is recorded only where scanning would find exactly that closer,
/// so jumping never changes what the lexer produces.
#[derive(Debug, Clone)]
pub struct BracketTable {
    /// The file's span, in the same global coordinates as every other [`Span`].
    pub covered_span: Span,
    /// For the byte at `covered_span.start + i`: if it is an opening bracket closed at
    /// `covered_span.start + j`, then `close[i] == j + 1`; otherwise `close[i] == 0`.
    pub close: Box<[u32]>,
}

impl BracketTable {
    /// The position of the closer of the opening bracket at `open`, if it is known.
    #[inline]
    pub fn close_of(&self, open: usize) -> Option<usize> {
        let index = open.checked_sub(self.covered_span.start)?;
        match self.close.get(index) {
            Some(&close) if close != 0 => Some(self.covered_span.start + close as usize - 1),
            _ => None,
        }
    }
}
