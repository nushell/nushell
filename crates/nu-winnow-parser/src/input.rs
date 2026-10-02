//! The winnow stream type used by the lexer and the literal parsers, and the
//! error type of every parser.
//!
//! [`Input`] is a character-level stream over a slice of the source. It
//! carries the absolute byte offset of the slice so every span produced from
//! it is absolute, even when the slice is the interior of a nested `[...]`.
//!
//! Token-level parsers read a `Tokens` stream instead (`src/parser/tokens.rs`).
//! Both kinds of parser fail with a [`ParseFailure`]: winnow's [`ErrMode`]
//! tells a *backtrack* (this parser did not match here, and an alternative
//! may) from a *cut* (a real error). Combinators backtrack all the time while
//! they try alternatives, so a backtrack carries nothing but its position; a
//! cut carries the boxed [`Diagnostic`] that will be reported. Keeping the
//! error small keeps every [`ParseResult`] cheap to return.

use winnow::error::{AddContext, ErrMode, FromExternalError, ModalResult, ParserError};
use winnow::stream::{LocatingSlice, Location, Stateful, Stream};

use crate::error::{Diagnostic, ErrorKind};
use crate::span::Span;

/// Absolute byte offset of the first byte of an [`Input`] slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Base(pub usize);

/// Character-level stream with absolute positions.
pub type Input<'a> = Stateful<LocatingSlice<&'a str>, Base>;

/// Why a parser failed.
#[derive(Debug)]
pub enum ParseFailure {
    /// Nothing matched at this absolute byte offset (a backtrack).
    NoMatch(usize),
    /// An error to report (a cut).
    Error(Box<Diagnostic>),
}

impl ParseFailure {
    /// The diagnostic to report. A stray [`ParseFailure::NoMatch`] becomes
    /// "expected valid syntax" at its offset.
    pub fn into_diagnostic(self) -> Diagnostic {
        match self {
            ParseFailure::NoMatch(offset) => Diagnostic::expected("valid syntax", Span::point(offset)),
            ParseFailure::Error(diagnostic) => *diagnostic,
        }
    }

    /// Record the grammar construct that was being parsed (innermost first).
    pub fn with_context(self, context: &'static str) -> Self {
        self.map_diagnostic(|diagnostic| diagnostic.with_context(context))
    }

    /// Change the diagnostic of an error; a backtrack is left as it is.
    pub fn map_diagnostic(self, change: impl FnOnce(Diagnostic) -> Diagnostic) -> Self {
        match self {
            ParseFailure::Error(diagnostic) => ParseFailure::Error(Box::new(change(*diagnostic))),
            no_match => no_match,
        }
    }

    /// Where the failure happened.
    fn offset(&self) -> usize {
        match self {
            ParseFailure::NoMatch(offset) => *offset,
            ParseFailure::Error(diagnostic) => diagnostic.span.start,
        }
    }
}

impl From<Diagnostic> for ParseFailure {
    fn from(diagnostic: Diagnostic) -> Self {
        ParseFailure::Error(Box::new(diagnostic))
    }
}

/// The result type used by every parser in this crate.
pub type ParseResult<T> = ModalResult<T, ParseFailure>;

/// Create a character stream over `text`, whose first byte lives at absolute offset `base`.
#[inline]
pub fn input(text: &str, base: usize) -> Input<'_> {
    Stateful { input: LocatingSlice::new(text), state: Base(base) }
}

/// Absolute byte offset of the next character.
#[inline]
pub fn pos(input: &Input<'_>) -> usize {
    input.state.0 + input.current_token_start()
}

/// The span from `start` to the current position.
#[inline]
pub fn span_from(input: &Input<'_>, start: usize) -> Span {
    Span::new(start, pos(input))
}

/// The absolute span of the remaining input.
#[inline]
pub fn rest_span(input: &Input<'_>) -> Span {
    let position = pos(input);
    Span::new(position, position + input.len())
}

/// A fatal (non-backtracking) error.
#[inline]
pub fn cut(diagnostic: Diagnostic) -> ErrMode<ParseFailure> {
    ErrMode::Cut(ParseFailure::from(diagnostic))
}

/// A recoverable error at absolute byte offset `offset`: `alt` will try the next branch.
#[inline]
pub fn backtrack(offset: usize) -> ErrMode<ParseFailure> {
    ErrMode::Backtrack(ParseFailure::NoMatch(offset))
}

impl<'a> ParserError<Input<'a>> for ParseFailure {
    type Inner = ParseFailure;

    #[inline]
    fn from_input(input: &Input<'a>) -> Self {
        ParseFailure::NoMatch(pos(input))
    }

    fn or(self, other: Self) -> Self {
        // Prefer the failure that got furthest.
        if other.offset() >= self.offset() { other } else { self }
    }

    fn into_inner(self) -> Result<Self::Inner, Self> {
        Ok(self)
    }
}

impl<'a> AddContext<Input<'a>, &'static str> for ParseFailure {
    fn add_context(
        self,
        _input: &Input<'a>,
        _start: &<Input<'a> as Stream>::Checkpoint,
        context: &'static str,
    ) -> Self {
        self.with_context(context)
    }
}

impl<'a, E: std::fmt::Display> FromExternalError<Input<'a>, E> for ParseFailure {
    fn from_external_error(input: &Input<'a>, error: E) -> Self {
        ParseFailure::from(Diagnostic::message(error.to_string(), Span::point(pos(input))))
    }
}

/// The diagnostic of a failed parse (an incomplete-input error cannot happen,
/// since all input is complete).
pub fn into_diagnostic(error: ErrMode<ParseFailure>) -> Diagnostic {
    match error {
        ErrMode::Backtrack(failure) | ErrMode::Cut(failure) => failure.into_diagnostic(),
        ErrMode::Incomplete(_) => Diagnostic::new(ErrorKind::UnexpectedEof("more input"), Span::point(0)),
    }
}
