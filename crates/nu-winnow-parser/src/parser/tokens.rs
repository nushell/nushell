//! A winnow stream over lexed tokens, and the token-level parsers.
//!
//! nu-parser walks the spans of a command by index. Here the items of one
//! command, or of a bracketed interior, are a [`Tokens`] stream that winnow's
//! combinators drive: it implements winnow's [`Stream`] and carries the
//! [`WorkingSet`], so a parser over tokens is a plain
//! `fn(&mut Tokens<'_, 'a>) -> ParseResult<T>` that composes with `opt`,
//! `repeat`, `separated`, `preceded`, `alt` and the rest.
//!
//! Besides its position the stream remembers where the tokens *end* in the
//! source, so that an error such as "expected block" can point just past the
//! last token when the input ran out.
//!
//! The parsers at the bottom of this file are the vocabulary the others are
//! written in: [`item`], [`keyword`], [`pipe`], [`eol`] and [`comment`] each
//! match one token and backtrack otherwise, and [`expected`] commits to a
//! parser, turning "did not match" into an `expected ...` error.

use std::fmt;
use std::iter::Enumerate;
use std::num::NonZeroUsize;
use std::ops::Range;
use std::slice::Iter;

use winnow::Parser;
use winnow::combinator::{eof, repeat_till};
use winnow::error::{AddContext, ErrMode, FromExternalError, Needed, ParserError};
use winnow::stream::{Location, Offset, Stream, StreamIsPartial};

use crate::error::{Diagnostic, ErrorKind};
use crate::input::{ParseFailure, ParseResult, backtrack, cut};
use crate::lex::{Token, TokenContents};
use crate::span::Span;

use super::WorkingSet;

/// A stream of tokens. Cheap to copy; copies are independent positions.
#[derive(Clone, Copy)]
pub struct Tokens<'t, 'a> {
    /// The working set of the parse.
    pub working_set: &'t WorkingSet<'a>,
    tokens: &'t [Token],
    position: usize,
    /// Byte offset just past the last token, for errors at the end.
    end: usize,
}

impl<'t, 'a> Tokens<'t, 'a> {
    /// A stream over `tokens`, which end at byte offset `end` in the source.
    pub fn new(working_set: &'t WorkingSet<'a>, tokens: &'t [Token], end: usize) -> Self {
        Self { working_set, tokens, position: 0, end }
    }

    /// A stream over the output of the lexer, whose last token is `Eof`.
    pub fn from_lexed(working_set: &'t WorkingSet<'a>, tokens: &'t [Token]) -> Self {
        match tokens.split_last() {
            Some((eof, rest)) if eof.contents == TokenContents::Eof => Self::new(working_set, rest, eof.span.start),
            _ => Self::new(working_set, tokens, tokens.last().map_or(0, |token| token.span.end)),
        }
    }

    /// The source text of a token.
    #[inline]
    pub fn text(&self, token: &Token) -> &'a str {
        self.working_set.get_span_contents(token.span)
    }

    /// The next token, without consuming it.
    #[inline]
    pub fn peek_token(&self) -> Option<&'t Token> {
        self.tokens.get(self.position)
    }

    /// Consume and return the next token.
    #[inline]
    pub fn next_token(&mut self) -> Option<&'t Token> {
        let token = self.tokens.get(self.position)?;
        self.position += 1;
        Some(token)
    }

    /// `true` when every token has been consumed.
    #[inline]
    pub fn at_end(&self) -> bool {
        self.position >= self.tokens.len()
    }

    /// The tokens not yet consumed.
    #[inline]
    pub fn remaining(&self) -> &'t [Token] {
        &self.tokens[self.position.min(self.tokens.len())..]
    }

    /// All tokens, consumed or not.
    #[inline]
    pub fn all(&self) -> &'t [Token] {
        self.tokens
    }

    /// The current position (an index into [`Tokens::all`]).
    #[inline]
    pub fn position(&self) -> usize {
        self.position
    }

    /// Move back (or forward) to a position obtained from [`Tokens::position`].
    #[inline]
    pub fn reset_to(&mut self, position: usize) {
        self.position = position;
    }

    /// The span of the next token, or the empty span at the end.
    #[inline]
    pub fn here(&self) -> Span {
        self.peek_token().map_or(Span::point(self.end), |token| token.span)
    }

    /// The empty span at the end of the tokens.
    #[inline]
    pub fn end_span(&self) -> Span {
        Span::point(self.end)
    }

    /// The span from the first to the last token, if any.
    pub fn span(&self) -> Option<Span> {
        Some(self.tokens.first()?.span.merge(self.tokens.last()?.span))
    }

    /// Consume the next token if it is an item; otherwise fail with `expected <what>`.
    #[inline]
    pub fn expect_item(&mut self, what: &'static str) -> ParseResult<Token> {
        match self.peek_token() {
            Some(token) if token.contents == TokenContents::Item => {
                self.position += 1;
                Ok(*token)
            }
            _ => Err(cut(Diagnostic::expected(what, self.here()))),
        }
    }

    /// Fail with "extra tokens" unless everything has been consumed.
    pub fn expect_end(&self) -> ParseResult<()> {
        match self.peek_token() {
            Some(token) => Err(cut(Diagnostic::new(ErrorKind::ExtraTokens, token.span))),
            None => Ok(()),
        }
    }

    /// Consume everything that is left and return the span it covers.
    pub fn consume_rest(&mut self) -> Option<Span> {
        let rest = self.remaining();
        let span = Some(rest.first()?.span.merge(rest.last()?.span));
        self.position = self.tokens.len();
        span
    }

    /// A stream over the tokens in `range` (indices into [`Tokens::all`]),
    /// ending where the token after the range starts.
    pub fn slice(&self, range: Range<usize>) -> Tokens<'t, 'a> {
        let end = self.tokens.get(range.end).map_or(self.end, |token| token.span.start);
        Tokens { working_set: self.working_set, tokens: &self.tokens[range], position: 0, end }
    }

    /// A stream over what is left, as an independent slice.
    pub fn rest_stream(&self) -> Tokens<'t, 'a> {
        self.slice(self.position.min(self.tokens.len())..self.tokens.len())
    }

    /// The token right after byte offset `offset` (the operator after its left
    /// operand, for instance).
    pub fn token_after(&self, offset: usize) -> Option<&'t Token> {
        let index = self.tokens.partition_point(|token| token.span.start < offset);
        self.tokens.get(index)
    }
}

impl fmt::Debug for Tokens<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tokens").field("remaining", &self.remaining()).field("end", &self.end).finish()
    }
}

/// A position in a [`Tokens`] stream, for winnow's backtracking.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TokenPosition(usize);

impl Offset for TokenPosition {
    #[inline]
    fn offset_from(&self, start: &Self) -> usize {
        self.0 - start.0
    }
}

impl Offset for Tokens<'_, '_> {
    #[inline]
    fn offset_from(&self, start: &Self) -> usize {
        self.position - start.position
    }
}

impl Offset<TokenPosition> for Tokens<'_, '_> {
    #[inline]
    fn offset_from(&self, start: &TokenPosition) -> usize {
        self.position - start.0
    }
}

impl<'t> Stream for Tokens<'t, '_> {
    type Token = &'t Token;
    type Slice = &'t [Token];
    type IterOffsets = Enumerate<Iter<'t, Token>>;
    type Checkpoint = TokenPosition;

    #[inline]
    fn iter_offsets(&self) -> Self::IterOffsets {
        self.remaining().iter().enumerate()
    }

    #[inline]
    fn eof_offset(&self) -> usize {
        self.tokens.len().saturating_sub(self.position)
    }

    #[inline]
    fn next_token(&mut self) -> Option<Self::Token> {
        Tokens::next_token(self)
    }

    #[inline]
    fn peek_token(&self) -> Option<Self::Token> {
        Tokens::peek_token(self)
    }

    #[inline]
    fn offset_for<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Token) -> bool,
    {
        self.remaining().iter().position(predicate)
    }

    #[inline]
    fn offset_at(&self, tokens: usize) -> Result<usize, Needed> {
        match tokens.checked_sub(self.eof_offset()).and_then(NonZeroUsize::new) {
            Some(needed) => Err(Needed::Size(needed)),
            None => Ok(tokens),
        }
    }

    #[inline]
    fn next_slice(&mut self, offset: usize) -> Self::Slice {
        let slice = &self.tokens[self.position..self.position + offset];
        self.position += offset;
        slice
    }

    #[inline]
    fn peek_slice(&self, offset: usize) -> Self::Slice {
        &self.tokens[self.position..self.position + offset]
    }

    #[inline]
    fn checkpoint(&self) -> Self::Checkpoint {
        TokenPosition(self.position)
    }

    #[inline]
    fn reset(&mut self, checkpoint: &Self::Checkpoint) {
        self.position = checkpoint.0;
    }

    fn trace(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.remaining())
    }
}

impl StreamIsPartial for Tokens<'_, '_> {
    type PartialState = ();

    #[inline]
    fn complete(&mut self) -> Self::PartialState {}

    #[inline]
    fn restore_partial(&mut self, _state: Self::PartialState) {}

    #[inline]
    fn is_partial_supported() -> bool {
        false
    }
}

/// Byte offsets in the source, so that winnow's `.with_span()` and `.span()`
/// give source ranges.
impl Location for Tokens<'_, '_> {
    #[inline]
    fn previous_token_end(&self) -> usize {
        match self.position.checked_sub(1).and_then(|index| self.tokens.get(index)) {
            Some(token) => token.span.end,
            None => self.here().start,
        }
    }

    #[inline]
    fn current_token_start(&self) -> usize {
        self.here().start
    }
}

impl<'t, 'a: 't> ParserError<Tokens<'t, 'a>> for ParseFailure {
    type Inner = ParseFailure;

    #[inline]
    fn from_input(tokens: &Tokens<'t, 'a>) -> Self {
        ParseFailure::NoMatch(tokens.here().start)
    }

    fn or(self, other: Self) -> Self {
        // Prefer the failure that got furthest.
        let offset = |failure: &ParseFailure| match failure {
            ParseFailure::NoMatch(offset) => *offset,
            ParseFailure::Error(diagnostic) => diagnostic.span.start,
        };
        if offset(&other) >= offset(&self) { other } else { self }
    }

    fn into_inner(self) -> Result<Self::Inner, Self> {
        Ok(self)
    }
}

impl<'t, 'a: 't> AddContext<Tokens<'t, 'a>, &'static str> for ParseFailure {
    fn add_context(self, _tokens: &Tokens<'t, 'a>, _start: &TokenPosition, context: &'static str) -> Self {
        self.with_context(context)
    }
}

impl<'t, 'a: 't, E: fmt::Display> FromExternalError<Tokens<'t, 'a>, E> for ParseFailure {
    fn from_external_error(tokens: &Tokens<'t, 'a>, error: E) -> Self {
        ParseFailure::from(Diagnostic::message(error.to_string(), tokens.here()))
    }
}

/// The next token if `accept` holds for it; otherwise backtrack.
#[inline]
fn token_where(tokens: &mut Tokens<'_, '_>, accept: impl Fn(&Token) -> bool) -> ParseResult<Token> {
    match tokens.peek_token() {
        Some(token) if accept(token) => {
            tokens.position += 1;
            Ok(*token)
        }
        _ => Err(backtrack(tokens.here().start)),
    }
}

/// Any item (a word, literal, bracketed group, ...).
#[inline]
pub fn item(tokens: &mut Tokens<'_, '_>) -> ParseResult<Token> {
    token_where(tokens, |token| token.contents == TokenContents::Item)
}

/// An item spelled exactly `word`, such as `else`, `in` or `=>`.
#[inline]
pub fn keyword<'t, 'a: 't>(word: &'static str) -> impl Parser<Tokens<'t, 'a>, Token, ErrMode<ParseFailure>> {
    move |tokens: &mut Tokens<'t, 'a>| {
        let working_set = tokens.working_set;
        token_where(tokens, |token| {
            token.contents == TokenContents::Item && working_set.get_span_contents(token.span) == word
        })
    }
}

/// A `|`.
#[inline]
pub fn pipe(tokens: &mut Tokens<'_, '_>) -> ParseResult<Token> {
    token_where(tokens, |token| token.contents == TokenContents::Pipe)
}

/// An end of line.
#[inline]
pub fn eol(tokens: &mut Tokens<'_, '_>) -> ParseResult<Token> {
    token_where(tokens, |token| token.contents == TokenContents::Eol)
}

/// A `# comment`.
#[inline]
pub fn comment(tokens: &mut Tokens<'_, '_>) -> ParseResult<Token> {
    token_where(tokens, |token| token.contents == TokenContents::Comment)
}

/// Commit to `parser`: where it does not match, fail with `expected <what>`
/// at the next token (or at the end), and let no alternative be tried.
#[inline]
pub fn expected<'t, 'a: 't, O>(
    what: &'static str,
    parser: impl Parser<Tokens<'t, 'a>, O, ErrMode<ParseFailure>>,
) -> impl Parser<Tokens<'t, 'a>, O, ErrMode<ParseFailure>> {
    cut_with(parser, move |tokens| Diagnostic::expected(what, tokens.here()))
}

/// Commit to `parser`: where it does not match, fail with the diagnostic
/// `error` builds from the stream (positioned where `parser` started).
#[inline]
pub fn cut_with<'t, 'a: 't, O>(
    mut parser: impl Parser<Tokens<'t, 'a>, O, ErrMode<ParseFailure>>,
    error: impl Fn(&Tokens<'t, 'a>) -> Diagnostic,
) -> impl Parser<Tokens<'t, 'a>, O, ErrMode<ParseFailure>> {
    move |tokens: &mut Tokens<'t, 'a>| {
        let start = tokens.position;
        match parser.parse_next(tokens) {
            Err(ErrMode::Backtrack(_)) => {
                tokens.position = start;
                Err(cut(error(tokens)))
            }
            result => result,
        }
    }
}

/// `parser` repeated until the tokens run out. Each repetition must match:
/// wrap `parser` in [`expected`] so that a token it does not match is an error.
#[inline]
pub fn repeat_to_end<'t, 'a: 't, O>(
    parser: impl Parser<Tokens<'t, 'a>, O, ErrMode<ParseFailure>>,
) -> impl Parser<Tokens<'t, 'a>, Vec<O>, ErrMode<ParseFailure>> {
    repeat_till(0.., parser, eof).map(|(items, _): (Vec<O>, _)| items)
}

/// The tokens up to (not including) the item spelled `word`, or up to the
/// end, as a stream of their own; the stream is left at `word`.
#[inline]
pub fn tokens_until<'t, 'a: 't>(
    word: &'static str,
) -> impl Parser<Tokens<'t, 'a>, Tokens<'t, 'a>, ErrMode<ParseFailure>> {
    move |tokens: &mut Tokens<'t, 'a>| {
        let working_set = tokens.working_set;
        let start = tokens.position;
        let length = tokens.remaining().iter().position(|token| {
            token.contents == TokenContents::Item && working_set.get_span_contents(token.span) == word
        });
        let end = length.map_or(tokens.tokens.len(), |length| start + length);
        tokens.position = end;
        Ok(tokens.slice(start..end))
    }
}
