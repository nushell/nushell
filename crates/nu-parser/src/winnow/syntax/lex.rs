//! The lexer (nu-parser's `lex.rs`: `lex`, `lex_n_tokens`, `lex_item`,
//! `Token`, `TokenContents`).
//!
//! Nushell is lexed into *items*: maximal runs of non-whitespace text in which
//! brackets (`()`, `[]`, `{}`) and quotes are balanced. `[1 2 3]` is a single
//! item, and so is `$x.a.b` or `foo(bar)`. Nested constructs are lexed again
//! from the interior of the item when they are parsed. This mirrors the
//! reference implementation in `nu-parser`, which is what gives Nushell its
//! whitespace-sensitive semantics (`1+1` is a bare word, `1 + 1` is math).
//!
//! Besides items the lexer produces pipes, redirection operators, semicolons,
//! end-of-line markers, comments and assignment operators. All spans are
//! absolute byte offsets into the original source.
//!
//! The lexer reads a character stream ([`crate::winnow::syntax::input::Input`]): [`next_token`]
//! skips whitespace and dispatches on the next byte. An item is measured by
//! `item_length`, a byte loop like nu's `lex_item`, because an item's end
//! depends on the quotes and brackets open at each byte rather than on a
//! grammar; [`GroupEnds`] lets it jump over a bracket group it measured before.

use winnow::stream::{Location, Stream};

use crate::winnow::syntax::error::{Diagnostic, ErrorKind};
use crate::winnow::syntax::input::{
    Input, ParseResult, cut, input, into_diagnostic, pos, span_from,
};
use crate::winnow::syntax::span::Span;

/// Which stream a redirection applies to and where it goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RedirectionOperator {
    /// `o>` / `out>`
    Out,
    /// `o>>` / `out>>`
    OutAppend,
    /// `e>` / `err>`
    Err,
    /// `e>>` / `err>>`
    ErrAppend,
    /// `o+e>` / `out+err>` / `e+o>` / `err+out>`
    OutErr,
    /// `o+e>>` and friends
    OutErrAppend,
    /// `e>|` / `err>|`
    ErrPipe,
    /// `o+e>|` and friends
    OutErrPipe,
}

impl RedirectionOperator {
    /// `true` for `>>` (append) variants.
    pub fn is_append(self) -> bool {
        matches!(
            self,
            RedirectionOperator::OutAppend
                | RedirectionOperator::ErrAppend
                | RedirectionOperator::OutErrAppend
        )
    }

    /// `true` for `>|` variants, which redirect into the next pipeline element.
    pub fn is_pipe(self) -> bool {
        matches!(
            self,
            RedirectionOperator::ErrPipe | RedirectionOperator::OutErrPipe
        )
    }

    /// Which stream(s) are redirected.
    pub fn source(self) -> RedirectionSource {
        match self {
            RedirectionOperator::Out | RedirectionOperator::OutAppend => RedirectionSource::Stdout,
            RedirectionOperator::Err
            | RedirectionOperator::ErrAppend
            | RedirectionOperator::ErrPipe => RedirectionSource::Stderr,
            RedirectionOperator::OutErr
            | RedirectionOperator::OutErrAppend
            | RedirectionOperator::OutErrPipe => RedirectionSource::StdoutAndStderr,
        }
    }
}

/// The stream a redirection reads from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RedirectionSource {
    /// `o>`
    Stdout,
    /// `e>`
    Stderr,
    /// `o+e>`
    StdoutAndStderr,
}

/// Assignment operators. These are lexed specially because they make the rest
/// of the line (pipes included) belong to the assignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AssignmentOperator {
    /// `=`
    Assign,
    /// `+=`
    AddAssign,
    /// `-=`
    SubtractAssign,
    /// `*=`
    MultiplyAssign,
    /// `/=`
    DivideAssign,
    /// `++=`
    ConcatenateAssign,
}

impl AssignmentOperator {
    /// The source spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            AssignmentOperator::Assign => "=",
            AssignmentOperator::AddAssign => "+=",
            AssignmentOperator::SubtractAssign => "-=",
            AssignmentOperator::MultiplyAssign => "*=",
            AssignmentOperator::DivideAssign => "/=",
            AssignmentOperator::ConcatenateAssign => "++=",
        }
    }
}

/// The kind of a lexed token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenContents {
    /// A bracket-balanced run of text: a word, literal, list, block, ...
    Item,
    /// `# ...` to the end of the line (without the newline).
    Comment,
    /// `|`
    Pipe,
    /// `||`
    PipePipe,
    /// `;`
    Semicolon,
    /// A newline.
    Eol,
    /// An assignment operator standing alone.
    AssignmentOperator(AssignmentOperator),
    /// A redirection operator standing alone.
    Redirection(RedirectionOperator),
    /// End of the token stream (always the last token).
    Eof,
}

/// A token with its absolute span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    /// What it is.
    pub contents: TokenContents,
    /// Where it is.
    pub span: Span,
}

impl Token {
    /// The token's text.
    #[inline]
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        self.span.slice(source)
    }

    /// `true` for an [`TokenContents::Item`].
    #[inline]
    pub fn is_item(&self) -> bool {
        self.contents == TokenContents::Item
    }
}

/// Options controlling how a piece of text is lexed.
///
/// Nested constructs lex their interior with different delimiters: lists treat
/// `,` and newlines as whitespace, records additionally split on `:`, cell
/// paths split on `.`, and so on. The constants below are the only options;
/// each carries the byte sets the lexer stops at, computed at compile time.
#[derive(Clone, Copy, Debug)]
pub struct LexOptions {
    /// Drop comments instead of emitting [`TokenContents::Comment`]. Only a probe
    /// (`BRACE_PROBE`) and text that never holds a comment nu accepts (`CELL_PATH`, `VAR_TYPE`)
    /// set it: every other comment must reach `Ast::comments`, so a parser that does not want
    /// comment tokens records and drops them.
    skip_comments: bool,
    /// The bytes the lexer treats specially and whether `<`/`>` nest, computed at compile time
    /// by `StopBytes::new` from the preset's extra whitespace and special tokens.
    stops: &'static StopBytes,
}

impl LexOptions {
    /// A preset: its byte sets, `stops`, which a constant below builds with `StopBytes::new`
    /// and borrows (so they live in the program), and whether it drops comments.
    const fn with_stops(stops: &'static StopBytes, skip_comments: bool) -> Self {
        LexOptions {
            skip_comments,
            stops,
        }
    }

    /// The options used for blocks and the top level of a file.
    pub const BLOCK: LexOptions =
        LexOptions::with_stops(&StopBytes::new(&[], &[], LexMode::Code), false);
    /// Options for subexpressions `( ... )`: newlines are whitespace, so a
    /// parenthesised pipeline may span several lines.
    pub const SUBEXPRESSION: LexOptions =
        LexOptions::with_stops(&StopBytes::new(b"\n\r", &[], LexMode::Code), false);
    /// Options for list interiors: commas and newlines are whitespace, and a `]` that closes no
    /// `[` is an error (nu's `lex_list_items`, see `LexMode::ListItems`).
    pub const LIST: LexOptions =
        LexOptions::with_stops(&StopBytes::new(b"\n\r,", &[], LexMode::ListItems), false);
    /// Options for record interiors: like lists, and `:` is special.
    pub const RECORD_KEY: LexOptions =
        LexOptions::with_stops(&StopBytes::new(b"\n\r,", b":", LexMode::Code), false);
    /// Options for record values: like lists, but nothing is special.
    pub const RECORD_VALUE: LexOptions =
        LexOptions::with_stops(&StopBytes::new(b"\n\r,", &[], LexMode::Code), false);
    /// Options for signatures `[a: int, --flag(-f)]`.
    pub const SIGNATURE: LexOptions =
        LexOptions::with_stops(&StopBytes::new(b"\n\r", b":=,", LexMode::Signature), false);
    /// Options for type parameters `list<int>`, `oneof<a, b>`, `record<a: int>`
    /// (nu's `lex_signature` in `parse_type_params`): `:` and `,` are special.
    /// nu skips comments here; the parser records them and drops them.
    pub const TYPE_PARAMS: LexOptions =
        LexOptions::with_stops(&StopBytes::new(b"\n\r", b":,", LexMode::Signature), false);
    /// Options for the type of a declared variable `let x: record<a: int>`
    /// (nu's `lex_signature` in `parse_var_with_opt_type`): `<`/`>` pair up,
    /// so a stray `]` or `}` inside them is unbalanced.
    pub const VAR_TYPE: LexOptions =
        LexOptions::with_stops(&StopBytes::new(b"", b",", LexMode::Signature), true);
    /// Options for input/output type lists `[int -> string, nothing -> nothing]`.
    pub const IO_TYPES: LexOptions =
        LexOptions::with_stops(&StopBytes::new(b"\n\r,", &[], LexMode::Signature), false);
    /// Options for cell paths: `.`, `?` and `!` are special.
    pub const CELL_PATH: LexOptions =
        LexOptions::with_stops(&StopBytes::new(b"\n\r", b".?!", LexMode::Code), true);
    /// Options for match blocks: commas and newlines are whitespace. A `|` still lexes as a
    /// [`TokenContents::Pipe`], which separates the alternatives of an or-pattern (`1 | 2 => x`).
    pub const MATCH: LexOptions =
        LexOptions::with_stops(&StopBytes::new(b" \r\n,", &[], LexMode::Code), false);
    /// Options for the first two tokens of a `{...}` body, used to decide what it is.
    pub const BRACE_PROBE: LexOptions =
        LexOptions::with_stops(&StopBytes::new(b"\r\n\t", b":", LexMode::Code), true);
    /// Options for binary literals `0x[ff 00]`.
    pub const BINARY: LexOptions =
        LexOptions::with_stops(&StopBytes::new(b",\r\n", &[], LexMode::Code), false);
    /// Options for match list patterns.
    pub const PATTERN_LIST: LexOptions =
        LexOptions::with_stops(&StopBytes::new(b"\n\r,", &[], LexMode::Code), false);
    /// Options for record patterns.
    pub const PATTERN_RECORD: LexOptions =
        LexOptions::with_stops(&StopBytes::new(b"\n\r,", b":", LexMode::Code), false);
}

/// Where the bracket groups of a block's source close, as scanning its items found them.
///
/// The parser lexes the inside of a `[...]`, `{...}` or `(...)` again when it parses it, so
/// without this table every byte would be scanned once for each bracket around it. Scanning an
/// item records where each group in it closes, and a later scan that meets the same opening
/// bracket jumps to its close. A group's extent does not depend on what is around it: the scan
/// enters it in the same state (no quote or comment open) and its own bracket is innermost until
/// it closes. Signature scans, which also pair `<` and `>`, neither record nor jump.
#[derive(Debug, Default)]
pub struct GroupEnds {
    /// The first offset of the text covered.
    start: usize,
    /// The length of the text covered.
    len: usize,
    /// For each offset from `start`, one past the offset (from `start`) of the bracket closing
    /// the group that opens there. Storing it plus one lets the zero-filled table mean "unknown"
    /// everywhere. Allocated, one `u32` per byte covered, on the first record.
    ends: Vec<u32>,
}

impl GroupEnds {
    /// A table for the groups opening within `span`.
    pub fn new(span: Span) -> Self {
        Self {
            start: span.start,
            len: span.len(),
            ends: Vec::new(),
        }
    }

    /// The group opening at `open` closes at `close`. A group opening outside the covered text,
    /// or closing too far from `start` for a `u32`, is not recorded; scans then measure it again.
    fn record(&mut self, open: usize, close: usize) {
        let Some(index) = open
            .checked_sub(self.start)
            .filter(|&index| index < self.len)
        else {
            return;
        };
        let Ok(end) = u32::try_from(close - self.start + 1) else {
            return;
        };
        if self.ends.is_empty() {
            self.ends = vec![0; self.len];
        }
        self.ends[index] = end;
    }

    /// Where the group opening at `open` closes, if a scan found it.
    #[inline]
    fn close(&self, open: usize) -> Option<usize> {
        let end = *self.ends.get(open.checked_sub(self.start)?)?;
        (end > 0).then(|| self.start + end as usize - 1)
    }
}

/// Lex `text`, whose first byte is at absolute offset `base` (nu's `lex`).
///
/// The returned vector always ends with a [`TokenContents::Eof`] token whose span is
/// the empty span at the end of `text`.
pub fn lex(text: &str, base: usize, options: LexOptions) -> Result<Vec<Token>, Diagnostic> {
    lex_n_tokens(text, base, options, usize::MAX)
}

/// An error nu's lexer reports and goes on past, with what nu's error needs: the lexer ends the
/// item where nu's does and lexes on ([`lex_recovering`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LexError {
    /// A closer that closes no open bracket (nu's `Unbalanced`): the opener it would need (the
    /// innermost open one, else its own), the closer and its span. The item ends after it.
    Unbalanced {
        /// The opener the closer would need.
        open: &'static str,
        /// The closer.
        close: &'static str,
        /// The closer's span.
        span: Span,
        /// Where the innermost open bracket opened, if one is open.
        innermost: Option<usize>,
    },
    /// A quote, bracket or interpolation still open where the item ends (nu's `Unclosed`): the
    /// closer it needs, the opener nu labels and where the closer was expected. nu's help names
    /// the construct the text before the opener starts (its `delimiter_structure_hint`, over
    /// what was lexed before the opener).
    Unclosed {
        /// The closer that is missing.
        delimiter: &'static str,
        /// The opener nu labels.
        open: Span,
        /// Where the closer was expected.
        end: Span,
    },
    /// A raw string without its closing `'` and `#`s (nu's `Unclosed`, with `'` and the
    /// string's `#`s for the closer).
    UnclosedRawString {
        /// The number of `#`s.
        hashes: usize,
        /// `r#'`, the opener.
        open: Span,
        /// The item's last byte.
        end: Span,
    },
    /// `r#` without a `'` after it (nu's `Expected` "'"), at the byte after the `#`s, where
    /// the item ends.
    RawStringQuote(Span),
    /// `&&` (nu's `ShellAndAnd`), which is a `|`.
    ShellAndAnd(Span),
    /// `2>` (nu's `ShellErrRedirect`), which is `e>`.
    ShellErrRedirect(Span),
    /// `2>&1` (nu's `ShellOutErrRedirect`), which is a `|`.
    ShellOutErrRedirect(Span),
    /// `o>|` or `out>|` (nu's `Expected`), which is a `|`.
    OutPipe(Span),
    /// A `;` right after a `|` (nu's `ExtraTokens`).
    SemicolonAfterPipe(Span),
}

impl LexError {
    /// The diagnostic of the error, for the parse's verdict (the consumer reports nu's).
    pub fn diagnostic(&self) -> Diagnostic {
        let shell = |found: &'static str, use_instead: &'static str, span: Span| {
            Diagnostic::new(ErrorKind::ShellSyntax { found, use_instead }, span)
        };
        match *self {
            LexError::Unbalanced {
                open,
                close,
                span,
                innermost,
            } => {
                let diagnostic = Diagnostic::new(
                    ErrorKind::Unbalanced {
                        found: close,
                        expected: open,
                    },
                    span,
                );
                match innermost {
                    Some(open_at) => diagnostic.with_help(format!(
                        "the innermost open delimiter is `{open}` at byte {open_at}"
                    )),
                    None => diagnostic,
                }
            }
            LexError::Unclosed {
                delimiter,
                open,
                end,
            } => Diagnostic::new(ErrorKind::Unclosed { delimiter, open }, end),
            LexError::UnclosedRawString { open, end, .. } => Diagnostic::new(
                ErrorKind::Unclosed {
                    delimiter: "'",
                    open,
                },
                end,
            ),
            LexError::RawStringQuote(span) => Diagnostic::expected("`'` after `r#`", span),
            LexError::ShellAndAnd(span) => shell("&&", ";", span)
                .with_help("use `;` to run commands in sequence, or `and` for boolean logic"),
            LexError::ShellErrRedirect(span) => shell("2>", "e>", span),
            LexError::ShellOutErrRedirect(span) => shell("2>&1", "o+e>", span),
            LexError::OutPipe(span) => shell("o>|", "|", span)
                .with_help("redirecting stdout to a pipe is the same as normal piping"),
            LexError::SemicolonAfterPipe(span) => Diagnostic::new(ErrorKind::ExtraTokens, span)
                .with_help("a `|` must be followed by a command before the `;`"),
        }
    }
}

/// A text's tokens and the errors nu's lexer reports in it and goes on past, each with the span
/// of the token it is in, in order ([`lex_recovering`]).
#[derive(Debug, Default)]
pub struct Lexed {
    /// The tokens, `Eof` last.
    pub tokens: Vec<Token>,
    /// The errors, each with the span of its token.
    pub errors: Vec<(Span, LexError)>,
}

/// Like [`lex`], but stop after `max_tokens` tokens, not counting `Eof`
/// (nu's `lex_n_tokens`).
///
/// This is used to look at the first couple of tokens of a `{ ... }` body
/// without lexing all of it.
pub fn lex_n_tokens(
    text: &str,
    base: usize,
    options: LexOptions,
    max_tokens: usize,
) -> Result<Vec<Token>, Diagnostic> {
    lex_n_tokens_with(text, base, options, max_tokens, &mut GroupEnds::default())
}

/// [`lex_n_tokens`], jumping over the bracket groups in `groups` and recording the others.
pub fn lex_n_tokens_with(
    text: &str,
    base: usize,
    options: LexOptions,
    max_tokens: usize,
    groups: &mut GroupEnds,
) -> Result<Vec<Token>, Diagnostic> {
    let lexed = lex_recovering(text, base, options, max_tokens, groups)?;
    match lexed.errors.first() {
        Some((_, error)) => Err(error.diagnostic()),
        None => Ok(lexed.tokens),
    }
}

/// [`lex_n_tokens_with`], going on past the errors nu's lexer goes on past, as it does: each
/// item ends where nu's does and the tokens are nu's (`&&` is a `|`), and the errors are
/// returned with them. Fails only on an item nu's lexer cannot end either.
pub fn lex_recovering(
    text: &str,
    base: usize,
    options: LexOptions,
    max_tokens: usize,
    groups: &mut GroupEnds,
) -> Result<Lexed, Diagnostic> {
    let mut input = input(text, base);
    // An estimate of the token count (one per six bytes), never more than the budget and `Eof`.
    let capacity = (text.len() / 6).max(4).min(max_tokens.saturating_add(1));
    let mut tokens: Vec<Token> = Vec::with_capacity(capacity);
    // nu's `is_complete`: a `;` after a `|` with no item between them is
    // "extra tokens" (`ls |; ls`, and `{|x|; 1}`, whose parameter list ends
    // with a plain `|`). Newlines, comments and `||` leave it as it is.
    let mut after_pipe = false;
    let mut errors = Vec::new();
    while tokens.len() < max_tokens {
        match next_token_recovering(&mut input, options, groups) {
            Ok(Some((token, error))) => {
                if let Some(error) = error {
                    errors.push((token.span, *error));
                }
                match token.contents {
                    // A `&&` or `2>&1` is a `|` for this too, as in nu.
                    TokenContents::Pipe => after_pipe = true,
                    TokenContents::Semicolon if after_pipe => {
                        errors.push((token.span, LexError::SemicolonAfterPipe(token.span)));
                    }
                    TokenContents::Eol
                    | TokenContents::Comment
                    | TokenContents::PipePipe
                    | TokenContents::Semicolon => {}
                    _ => after_pipe = false,
                }
                tokens.push(token);
            }
            Ok(None) => break,
            Err(error) => return Err(into_diagnostic(error)),
        }
    }
    skip_whitespace(&mut input, options);
    tokens.push(Token {
        contents: TokenContents::Eof,
        span: Span::point(pos(&input)),
    });
    Ok(Lexed { tokens, errors })
}

/// The next token of `input` after whitespace, or `None` at its end. With a
/// preset that skips comments (`VAR_TYPE`, `CELL_PATH`, ...) a comment is
/// passed over. Bracket groups are measured with `groups` (see [`GroupEnds`]).
#[inline]
pub fn next_token(
    input: &mut Input<'_>,
    options: LexOptions,
    groups: &mut GroupEnds,
) -> ParseResult<Option<Token>> {
    match next_token_recovering(input, options, groups)? {
        Some((_, Some(error))) => Err(cut(error.diagnostic())),
        token => Ok(token.map(|(token, _)| token)),
    }
}

/// [`next_token`], with the error nu's lexer reports in the token and goes on past.
#[inline]
fn next_token_recovering(
    input: &mut Input<'_>,
    options: LexOptions,
    groups: &mut GroupEnds,
) -> ParseResult<Option<(Token, Option<Box<LexError>>)>> {
    loop {
        skip_whitespace(input, options);
        if input.is_empty() {
            return Ok(None);
        }
        if let Some(token) = lex_token(input, options, groups)? {
            return Ok(Some(token));
        }
    }
}

/// Skip the whitespace of `options` (space, tab, `\r` and the preset's own). A byte loop over
/// the [`ByteSet`] rather than winnow's `take_while`, whose tokens on this `&str` stream are
/// decoded `char`s: it runs before every token.
fn skip_whitespace(input: &mut Input<'_>, options: LexOptions) {
    let bytes = input.input.as_ref().as_bytes();
    let mut whitespace = 0;
    while let Some(&byte) = bytes.get(whitespace)
        && options.stops.whitespace.contains(byte)
    {
        whitespace += 1;
    }
    input.next_slice(whitespace);
}

/// One token, dispatched on its first byte, with the error nu's lexer reports in it. `None` for
/// input that produces no token (a skipped comment).
fn lex_token(
    input: &mut Input<'_>,
    options: LexOptions,
    groups: &mut GroupEnds,
) -> ParseResult<Option<(Token, Option<Box<LexError>>)>> {
    let start = pos(input);
    let text: &str = input.input.as_ref();
    let mut error = None;
    let kind = match text.as_bytes() {
        [b'\n', ..] => {
            input.next_slice(1);
            Some(TokenContents::Eol)
        }
        [b'#', ..] => {
            lex_comment(input);
            (!options.skip_comments).then_some(TokenContents::Comment)
        }
        [b'|', b'|', ..] => {
            input.next_slice(2);
            Some(TokenContents::PipePipe)
        }
        [b'|', ..] => {
            input.next_slice(1);
            Some(TokenContents::Pipe)
        }
        [b';', ..] => {
            input.next_slice(1);
            Some(TokenContents::Semicolon)
        }
        _ => {
            let (kind, item_error) = lex_item(input, options, groups)?;
            error = item_error;
            Some(kind)
        }
    };
    Ok(kind.map(|kind| {
        let token = Token {
            contents: kind,
            span: span_from(input, start),
        };
        (token, error)
    }))
}

/// A comment runs to the newline; like nu, a `\r` before it is part of the comment.
fn lex_comment(input: &mut Input<'_>) {
    let text: &str = input.input.as_ref();
    input.next_slice(text.find('\n').unwrap_or(text.len()));
}

/// The opening bracket kinds tracked while scanning an item.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Bracket {
    /// `(`
    Paren,
    /// `[`
    Square,
    /// `{`
    Curly,
    /// `<`, paired only in a signature (`LexMode::Signature`).
    Angle,
}

impl Bracket {
    /// The spelling of this opening bracket, for error messages.
    fn opener(self) -> &'static str {
        match self {
            Bracket::Paren => "(",
            Bracket::Square => "[",
            Bracket::Curly => "{",
            Bracket::Angle => "<",
        }
    }

    /// The bracket that closes this one, for an unclosed-bracket error.
    fn closer(self) -> &'static str {
        match self {
            Bracket::Paren => ")",
            Bracket::Square => "]",
            Bracket::Curly => "}",
            Bracket::Angle => ">",
        }
    }
}

/// The spelling of the delimiter an unclosed-quote error asks for: the quote itself, or `)`, the
/// closer an interpolation subexpression `$"... (...)"` waits for (see `interp_level` in
/// [`item_length`]).
fn quote_str(quote: u8) -> &'static str {
    match quote {
        b'"' => "\"",
        b'\'' => "'",
        b'`' => "`",
        b')' => ")",
        _ => "?",
    }
}

/// Advance the delimiter matching inside a `(...)` subexpression of an
/// interpolated string by one `byte` at absolute offset `at` (nu's
/// `interp_subexpr_step`). [`item_length`] and the interpolation parser
/// (`parse_interpolation_parts`) share it, so both end the string at the same byte.
///
/// `stack` holds the closers still expected, each with where its opener is. While
/// the innermost is a quote only that quote closes it; otherwise quotes open nested
/// strings, `(` nests and `)` closes. Escapes exist only in double-quoted strings:
/// returns `true` when `byte` is a backslash inside a nested `"` string, in which
/// case the caller must skip the next byte.
pub(crate) fn interp_subexpr_step(stack: &mut Vec<(u8, usize)>, byte: u8, at: usize) -> bool {
    match stack.last() {
        Some(&(expected, _)) if expected != b')' => {
            if expected == b'"' && byte == b'\\' {
                return true;
            }
            if byte == expected {
                stack.pop();
            }
        }
        _ => match byte {
            b'\'' | b'"' | b'`' => stack.push((byte, at)),
            b'(' => stack.push((b')', at)),
            b')' => {
                stack.pop();
            }
            _ => {}
        },
    }
    false
}

/// Whether `text` is a redirection before a `|` (`e>|`), which then belongs
/// to the item (nu's `is_redirection`).
fn is_redirection(text: &[u8]) -> bool {
    matches!(
        text,
        b"o>" | b"out>" | b"e>" | b"err>" | b"o+e>" | b"e+o>" | b"out+err>" | b"err+out>"
    )
}

/// Lex one item at the current position (nu's `lex_item`): [`item_length`]
/// measures it, [`item_contents`] tells an operator spelling from a plain item,
/// and the stream advances past it.
fn lex_item(
    input: &mut Input<'_>,
    options: LexOptions,
    groups: &mut GroupEnds,
) -> ParseResult<(TokenContents, Option<Box<LexError>>)> {
    let text = *input.input.as_ref();
    let bytes = text.as_bytes();
    let base = input.state.0 + input.current_token_start();
    // An item nu's lexer reports an error in ends where nu's does, a plain item.
    let (kind, error, offset) = match item_length(bytes, base, options, false, groups) {
        Ok(offset) => {
            let span = Span::new(base, base + offset);
            let (kind, error) = item_contents(&bytes[..offset], span);
            (kind, error, offset)
        }
        Err(fault) => (TokenContents::Item, Some(fault.error), fault.length),
    };
    // An empty item would leave the stream where it is, and `lex_n_tokens_with` would loop.
    if offset == 0 {
        return Err(cut(Diagnostic::new(
            ErrorKind::UnexpectedEof("command"),
            Span::point(base),
        )));
    }
    input.next_slice(offset);
    Ok((kind, error))
}

/// Where nu's lexer ends an item it reports an error in (its length), and the error, boxed so
/// that the scan's result stays small: it is returned for every item.
struct Fault {
    length: usize,
    error: Box<LexError>,
}

/// The offset of the bracket closing the group that `text` opens with
/// (`(a)/b` gives 2), scanning exactly as the lexer does: quotes, raw
/// strings, comments and nested brackets are respected. `None` if `text`
/// does not start with a bracket or the group is not closed.
pub(crate) fn group_end(text: &str) -> Option<usize> {
    group_end_with(text, 0, &mut GroupEnds::default())
}

/// [`group_end`] for `text` at absolute offset `base`, measured with `groups`.
pub(crate) fn group_end_with(text: &str, base: usize, groups: &mut GroupEnds) -> Option<usize> {
    if !text.starts_with(['(', '[', '{']) {
        return None;
    }
    item_length(text.as_bytes(), base, LexOptions::BLOCK, true, groups)
        .ok()
        .map(|offset| offset - 1)
}

/// Scan the item at the start of `bytes` (whose first byte is at absolute
/// offset `base`) and return its length: the scan of nu's `lex_item`. With
/// `first_group` set, stop right after the bracket closing the group opened by
/// the first byte (see [`group_end`]).
///
/// The item ends at the first terminator (`StopBytes::terminators`: whitespace,
/// `|`, `;` and the options' own) met outside every quote, comment and bracket.
/// Where that is depends on what is open at each byte, so the scan is a byte loop
/// over this state rather than a grammar:
///
/// - `quote`: the quote the scan is inside (`'`, `"` or `` ` ``) and where it
///   opened. Only `"` has backslash escapes. `quote_is_interp` marks `$"..."` and
///   `$'...'`, in which a `(` opens a subexpression.
/// - `interp_level`: the delimiters open inside that subexpression, matched by
///   [`interp_subexpr_step`]. While it is not empty the string's own quote does
///   not close the string.
/// - `brackets`: the open `(`, `[`, `{` (and `<` in a signature), each with its
///   offset. Inside a bracket no terminator ends the item.
/// - `in_comment`: a `#` after whitespace starts a comment that runs to the next
///   newline and in which quotes and brackets do not count. In practice this
///   happens inside brackets, since outside them whitespace ends the item.
/// - `previous`: the last byte scanned, to see the `r` of a raw string
///   `r#'...'#`, the `$` of an interpolated string and the whitespace before a
///   comment's `#`.
///
/// Outside quotes, a run of bytes that changes nothing but `previous` is skipped
/// at once. At an opening bracket whose group `groups` knows, the scan jumps to
/// its closing bracket; each group it closes otherwise is recorded there. A
/// signature scan does neither, since pairing `<` and `>` can change a group's
/// extent. A stray `]` at depth zero is text; a stray `)` or `}` is an error, as
/// is a quote or bracket still open at the end.
fn item_length(
    bytes: &[u8],
    base: usize,
    options: LexOptions,
    first_group: bool,
    groups: &mut GroupEnds,
) -> Result<usize, Fault> {
    let absolute = |offset: usize| base + offset;

    // The quote we are inside, with the offset where it opened.
    let mut quote: Option<(u8, usize)> = None;
    let mut quote_is_interp = false;
    // Open delimiters inside a `(...)` of an interpolated string.
    let mut interp_level: Vec<(u8, usize)> = Vec::new();
    let mut in_comment = false;
    let mut brackets: Vec<(Bracket, usize)> = Vec::new();
    let mut previous: Option<u8> = None;
    let mut offset = 0usize;
    let stops = options.stops;
    let in_signature = stops.mode == LexMode::Signature;

    // nu's `is_item_terminator`: only at bracket depth zero does a terminator end the item.
    let is_terminator = |brackets: &[(Bracket, usize)], byte: u8| {
        brackets.is_empty() && stops.terminators.contains(byte)
    };

    while offset < bytes.len() {
        // Outside quotes most bytes (letters, digits, `$`, `-`, ...) are ones the match below
        // passes over, so skip the whole run in this tight loop: it runs for every byte of every
        // item. `StopBytes::item` lists the bytes that matter at depth zero, `group` those inside
        // brackets.
        if quote.is_none() {
            let stop = if brackets.is_empty() {
                &stops.item
            } else {
                &stops.group
            };
            let mut plain_end = offset;
            while plain_end < bytes.len() && !stop.contains(bytes[plain_end]) {
                plain_end += 1;
            }
            if plain_end > offset {
                offset = plain_end;
                previous = Some(bytes[offset - 1]);
                continue;
            }
        }
        let byte = bytes[offset];
        match quote {
            Some(_) if !interp_level.is_empty() => {
                // Inside `$"... ( ... )"`: track nested delimiters until the `)` closes. A
                // backslash as the last byte is not skipped past the end; the unclosed
                // subexpression is reported after the loop.
                let escaped = interp_subexpr_step(&mut interp_level, byte, absolute(offset))
                    && offset + 1 < bytes.len();
                offset += if escaped { 2 } else { 1 };
                previous = Some(byte);
                continue;
            }
            Some((open_quote, open_at)) => match byte {
                b'\\' if open_quote == b'"' => {
                    // A backslash ending the text: nu ends the item before it, and the
                    // backslash is the next item.
                    if offset + 1 >= bytes.len() {
                        return Err(Fault {
                            length: offset,
                            error: Box::new(LexError::Unclosed {
                                delimiter: quote_str(open_quote),
                                open: Span::new(open_at, open_at + 1),
                                end: last_byte(base, offset),
                            }),
                        });
                    }
                    offset += 2;
                    previous = Some(byte);
                    continue;
                }
                _ if byte == open_quote => quote = None,
                b'(' if quote_is_interp => interp_level.push((b')', absolute(offset))),
                _ => {}
            },
            None => match byte {
                // `r#'...'#`, a raw string; its `r` went by as an ordinary byte.
                b'#' if !in_comment && previous == Some(b'r') => {
                    offset = lex_raw_string(bytes, offset - 1, absolute)?;
                    previous = Some(b'#');
                    continue;
                }
                // nu's rule: a `#` after ASCII whitespace, vertical tab included, starts a comment.
                b'#' if !in_comment => {
                    in_comment = previous.is_none_or(|previous| {
                        previous.is_ascii() && char::from(previous).is_whitespace()
                    })
                }
                b'\n' | b'\r' => {
                    in_comment = false;
                    if is_terminator(&brackets, byte) {
                        break;
                    }
                }
                _ if in_comment => {
                    if is_terminator(&brackets, byte) {
                        break;
                    }
                }
                // A special character (`:` in record keys, `.` in cell paths) is an item of its own
                // (nu's `is_special_item`); elsewhere at depth zero it ends the item.
                _ if offset == 0 && brackets.is_empty() && stops.special_tokens.contains(byte) => {
                    offset += 1;
                    break;
                }
                b'\'' | b'"' | b'`' => {
                    quote = Some((byte, absolute(offset)));
                    quote_is_interp = byte != b'`' && previous == Some(b'$');
                }
                b'[' | b'{' | b'(' => {
                    // A group an earlier scan measured: on to its closing bracket, unless that lies
                    // past the end of `bytes` (this text ends inside the group).
                    if !in_signature
                        && let Some(close) = groups.close(absolute(offset))
                        && close < absolute(bytes.len())
                    {
                        offset = close - base + 1;
                        previous = Some(bytes[close - base]);
                        if first_group && brackets.is_empty() {
                            break;
                        }
                        continue;
                    }
                    let bracket = match byte {
                        b'[' => Bracket::Square,
                        b'{' => Bracket::Curly,
                        _ => Bracket::Paren,
                    };
                    brackets.push((bracket, absolute(offset)));
                }
                b'<' if in_signature => brackets.push((Bracket::Angle, absolute(offset))),
                // A `>` closes a `<` only when that is the innermost open bracket; otherwise it is
                // text, such as the arrow in `[int -> string]`.
                b'>' if in_signature => {
                    if matches!(brackets.last(), Some((Bracket::Angle, _))) {
                        brackets.pop();
                    }
                }
                b']' | b'}' | b')' => {
                    // nu ends an item at a closer that closes nothing.
                    let closed = close_bracket(&mut brackets, byte, absolute(offset), stops.mode)
                        .map_err(|error| Fault {
                        length: offset + 1,
                        error,
                    })?;
                    if let Some(open) = closed
                        && !in_signature
                    {
                        groups.record(open, absolute(offset));
                    }
                    if first_group && brackets.is_empty() {
                        offset += 1;
                        break;
                    }
                }
                // `e>|` is one token even though `|` normally terminates the item.
                b'|' if is_redirection(&bytes[..offset]) => {
                    offset += 1;
                    break;
                }
                _ if is_terminator(&brackets, byte) => break,
                _ => {}
            },
        }
        offset += 1;
        previous = Some(byte);
    }

    // Still open at the end: nu's item ends there, the closer expected at its last byte.
    let unclosed = |delimiter, open: Span, end| Fault {
        length: offset,
        error: Box::new(LexError::Unclosed {
            delimiter,
            open,
            end,
        }),
    };
    let end = last_byte(base, offset);
    if let Some(&(closer, open_at)) = interp_level.first() {
        return Err(unclosed(
            quote_str(closer),
            Span::new(open_at, open_at + 1),
            end,
        ));
    }
    if let Some((quote, open_at)) = quote {
        return Err(unclosed(
            quote_str(quote),
            Span::new(open_at, open_at + 1),
            end,
        ));
    }
    if let Some(&(open, open_at)) = brackets.last() {
        let (label_open, label_end) = match closer_label_hint(&bytes[..offset], base, stops) {
            Some((hint_open, expected)) if hint_open == open_at => (
                Span::new(hint_open, hint_open + 1),
                Span::new(expected, expected + 1),
            ),
            _ => (Span::new(open_at, open_at + 1), end),
        };
        return Err(unclosed(open.closer(), label_open, label_end));
    }
    Ok(offset)
}

/// The span of the last byte of the `length` bytes at `base` (empty when there are none), where
/// nu expects the closer an item is missing.
fn last_byte(base: usize, length: usize) -> Span {
    match length {
        0 => Span::point(base),
        _ => Span::new(base + length - 1, base + length),
    }
}

/// nu's `CloserLabelHint`, for an item still inside a bracket where it ends: where the closer
/// probably belongs, with the bracket it relates to, nu's label for the missing closer when that
/// bracket is the innermost one left open. That is the first `{` opened right inside a `[`
/// (with the `[`), or the first `|` that starts a line inside a `{` when the line before does
/// not end with a byte that continues it (with the innermost `{`). `bytes` is the item, at
/// absolute offset `base`; the scan is nu's `lex_item` over it again, without jumping over any
/// group, as nu scans an item with an error again. Only that error path runs it.
#[cold]
#[inline(never)]
fn closer_label_hint(bytes: &[u8], base: usize, stops: &StopBytes) -> Option<(usize, usize)> {
    // A byte that continues a construct onto the next line (nu's `continues_onto_next_line`).
    let continues = |byte: u8| b"|{([,:+-*/=.".contains(&byte);
    let absolute = |offset: usize| base + offset;
    let mut quote: Option<u8> = None;
    let mut quote_is_interp = false;
    let mut interp_level: Vec<(u8, usize)> = Vec::new();
    let mut in_comment = false;
    let mut brackets: Vec<(Bracket, usize)> = Vec::new();
    let mut previous: Option<u8> = None;
    let mut at_line_start = true;
    let mut prev_line_continue = false;
    let mut last_significant: Option<u8> = None;
    let mut hint = None;
    let in_signature = stops.mode == LexMode::Signature;
    let mut offset = 0;
    while let Some(&byte) = bytes.get(offset) {
        match quote {
            Some(_) if !interp_level.is_empty() => {
                let escaped = interp_subexpr_step(&mut interp_level, byte, absolute(offset))
                    && offset + 1 < bytes.len();
                if !escaped {
                    last_significant = Some(byte);
                }
                at_line_start = false;
                offset += if escaped { 2 } else { 1 };
                previous = Some(byte);
                continue;
            }
            Some(open_quote) => {
                if byte == b'\\' && open_quote == b'"' {
                    at_line_start = false;
                    offset += 2;
                    previous = Some(byte);
                    continue;
                }
                if byte == open_quote {
                    quote = None;
                } else if quote_is_interp && byte == b'(' {
                    interp_level.push((b')', absolute(offset)));
                }
                last_significant = Some(byte);
                at_line_start = false;
            }
            None => match byte {
                b'#' if !in_comment && previous == Some(b'r') => {
                    match lex_raw_string(bytes, offset - 1, absolute) {
                        Ok(end) => offset = end,
                        Err(_) => break,
                    }
                    last_significant = Some(b'#');
                    at_line_start = false;
                    previous = Some(b'#');
                    continue;
                }
                b'#' if !in_comment => {
                    in_comment = previous.is_none_or(|previous| {
                        previous.is_ascii() && char::from(previous).is_whitespace()
                    })
                }
                b'\n' | b'\r' => {
                    in_comment = false;
                    if brackets.is_empty() && stops.terminators.contains(byte) {
                        break;
                    }
                    if byte == b'\n' || bytes.get(offset + 1) != Some(&b'\n') {
                        prev_line_continue = last_significant.is_some_and(continues);
                        at_line_start = true;
                        last_significant = None;
                    }
                }
                _ if in_comment => {
                    if brackets.is_empty() && stops.terminators.contains(byte) {
                        break;
                    }
                }
                _ if offset == 0 && brackets.is_empty() && stops.special_tokens.contains(byte) => {
                    break;
                }
                b'|' if is_redirection(&bytes[..offset]) => break,
                _ if brackets.is_empty() && stops.terminators.contains(byte) => break,
                _ => {
                    match byte {
                        b'\'' | b'"' | b'`' => {
                            quote = Some(byte);
                            quote_is_interp = byte != b'`' && previous == Some(b'$');
                        }
                        b'{' => {
                            if hint.is_none()
                                && let Some(&(Bracket::Square, square)) = brackets.last()
                            {
                                hint = Some((square, absolute(offset)));
                            }
                            brackets.push((Bracket::Curly, absolute(offset)));
                        }
                        b'[' => brackets.push((Bracket::Square, absolute(offset))),
                        b'(' => brackets.push((Bracket::Paren, absolute(offset))),
                        b'<' if in_signature => brackets.push((Bracket::Angle, absolute(offset))),
                        b'>' if in_signature => {
                            if matches!(brackets.last(), Some((Bracket::Angle, _))) {
                                brackets.pop();
                            }
                        }
                        // The item reported no unbalanced closer, so each closes the innermost.
                        b']' | b'}' | b')' => {
                            brackets.pop();
                        }
                        b'|' if at_line_start && !prev_line_continue && hint.is_none() => {
                            if let Some(&(_, curly)) = brackets
                                .iter()
                                .rev()
                                .find(|(bracket, _)| *bracket == Bracket::Curly)
                            {
                                hint = Some((curly, absolute(offset)));
                            }
                        }
                        _ => {}
                    }
                    if !byte.is_ascii_whitespace() {
                        last_significant = Some(byte);
                        at_line_start = false;
                    } else if !(at_line_start && (byte == b' ' || byte == b'\t')) {
                        at_line_start = false;
                    }
                }
            },
        }
        offset += 1;
        previous = Some(byte);
    }
    hint
}

/// The bytes [`item_length`] must look at outside quotes, for one [`LexOptions`].
#[derive(Debug)]
struct StopBytes {
    /// At bracket depth zero: the terminators and whitespace (with the options' own), brackets
    /// (`<` and `>` in a signature), quotes and `#` (a comment, or the end of a raw string's `r#`).
    /// Every other byte just continues the item.
    item: ByteSet,
    /// Inside a bracket group, where whitespace, terminators and special tokens do not end the
    /// item and `|` cannot follow a redirection (the item so far holds the open bracket): only
    /// brackets, quotes, `#` and the newline that ends a comment.
    group: ByteSet,
    /// The bytes that end an item at depth zero.
    terminators: ByteSet,
    /// The options' special tokens.
    special_tokens: ByteSet,
    /// The bytes skipped between tokens: space, tab, `\r` and the options' own.
    whitespace: ByteSet,
    /// What the text is: in a signature `<` and `>` nest as brackets (type annotations such as
    /// `list<int>`), and such a scan neither records nor jumps over bracket groups (see
    /// [`GroupEnds`]); in a list's interior a stray `]` is an error.
    mode: LexMode,
}

/// What the lexer is reading, for the few rules that depend on it (nu's `LexMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LexMode {
    /// Code.
    Code,
    /// A signature or type, where `<` and `>` nest like brackets.
    Signature,
    /// The inside of a list literal, without its own brackets. A `]` that closes no `[` there
    /// would close the list's `[` before the list ends (`[1 2]]`), so it is an error instead of
    /// part of a bare word.
    ListItems,
}

impl StopBytes {
    /// The sets for one [`LexOptions`] preset:
    /// - `additional_whitespace`: extra bytes treated as whitespace (in addition to space, tab,
    ///   `\r`); including `\n` suppresses [`TokenContents::Eol`] tokens.
    /// - `special_tokens`: bytes that are emitted as single-character items when they start a
    ///   token and that terminate the item otherwise (e.g. `:` in records).
    /// - `mode`: what the text is ([`LexMode`]).
    const fn new(additional_whitespace: &[u8], special_tokens: &[u8], mode: LexMode) -> Self {
        let brackets: &[u8] = if matches!(mode, LexMode::Signature) {
            b"[]{}()<>"
        } else {
            b"[]{}()"
        };
        let terminators = ByteSet::EMPTY
            .with(b" \t\n\r|;")
            .with(additional_whitespace)
            .with(special_tokens);
        StopBytes {
            item: terminators.with(b"#'\"`").with(brackets),
            group: ByteSet::EMPTY.with(b"\n\r#'\"`").with(brackets),
            terminators,
            special_tokens: ByteSet::EMPTY.with(special_tokens),
            whitespace: ByteSet::EMPTY.with(b" \t\r").with(additional_whitespace),
            mode,
        }
    }
}

/// A set of bytes: a 256-bit map in which byte `b` is bit `b & 63` of word `b >> 6`. Built in
/// `const` context for each preset, so a membership test in the scan is one bit test.
#[derive(Clone, Copy, Debug)]
struct ByteSet([u64; 4]);

impl ByteSet {
    /// The set with no bytes.
    const EMPTY: ByteSet = ByteSet([0; 4]);

    /// This set with `bytes` added.
    const fn with(mut self, bytes: &[u8]) -> ByteSet {
        let mut index = 0;
        while index < bytes.len() {
            let byte = bytes[index];
            self.0[(byte >> 6) as usize] |= 1 << (byte & 63);
            index += 1;
        }
        self
    }

    /// Whether `byte` is in the set.
    #[inline]
    fn contains(&self, byte: u8) -> bool {
        self.0[(byte >> 6) as usize] & (1 << (byte & 63)) != 0
    }
}

/// Pop the bracket closed by `closer`, returning where it opened, or report the
/// mismatch. With no bracket open, a `]` is ordinary text (`a]`, the glob `[]]*`) and
/// gives `None`, except inside a list (`mode` [`LexMode::ListItems`]), where it would
/// close the list early (`[1 2]]`); a `}` or `)` is an error; a closer that does not
/// match the innermost open bracket (`(a]`) is always an error.
fn close_bracket(
    brackets: &mut Vec<(Bracket, usize)>,
    closer: u8,
    at: usize,
    mode: LexMode,
) -> Result<Option<usize>, Box<LexError>> {
    let (expected, found) = match closer {
        b']' => (Bracket::Square, "]"),
        b'}' => (Bracket::Curly, "}"),
        _ => (Bracket::Paren, ")"),
    };
    match brackets.last() {
        Some(&(open, open_at)) if open == expected => {
            brackets.pop();
            Ok(Some(open_at))
        }
        Some(&(open, open_at)) => Err(Box::new(LexError::Unbalanced {
            open: open.opener(),
            close: found,
            span: Span::new(at, at + 1),
            innermost: Some(open_at),
        })),
        None if closer == b']' && mode != LexMode::ListItems => Ok(None),
        None => Err(Box::new(LexError::Unbalanced {
            open: expected.opener(),
            close: found,
            span: Span::new(at, at + 1),
            innermost: None,
        })),
    }
}

/// Scan a raw string `r#'...'#` starting at the `r` (nu's `lex_raw_string`);
/// returns the offset just past it. The string ends at the first `'` followed by
/// as many `#`s as follow the `r`, so `r##'a'#'##` holds `a'#`. Like nu, that `'`
/// may be the opening quote itself: the raw part of `r#'#a'#` is `r#'#`, and the
/// `a'#` after it opens a quote that never closes.
fn lex_raw_string(
    bytes: &[u8],
    start: usize,
    absolute: impl Fn(usize) -> usize,
) -> Result<usize, Fault> {
    let mut hashes = 0;
    while bytes.get(start + 1 + hashes) == Some(&b'#') {
        hashes += 1;
    }
    let quote_at = start + 1 + hashes;
    // nu ends the item after the `#`s and reports the byte after them.
    if bytes.get(quote_at) != Some(&b'\'') {
        let at = absolute(quote_at);
        return Err(Fault {
            length: quote_at,
            error: Box::new(LexError::RawStringQuote(Span::new(at, at + 1))),
        });
    }
    // The closing `#`s are as many as the opening ones, which follow the `r`.
    let closing_hashes = &bytes[start + 1..quote_at];
    // nu's `lex_raw_string` looks for the closing quote from the opening one on (a quirk).
    let mut offset = quote_at;
    while offset < bytes.len() {
        if bytes[offset] == b'\'' && bytes[offset + 1..].starts_with(closing_hashes) {
            return Ok(offset + 1 + hashes);
        }
        offset += 1;
    }
    // nu's item runs to the end of the text.
    Err(Fault {
        length: bytes.len(),
        error: Box::new(LexError::UnclosedRawString {
            hashes,
            open: Span::new(absolute(start), absolute(quote_at + 1)),
            end: last_byte(absolute(0), bytes.len()),
        }),
    })
}

/// Whether `text` is an operator that redirects a stream to a file (`o>`, `e>>`, `2>`, ...),
/// which takes the item after it as its target.
pub(crate) fn is_file_redirection_operator(text: &str) -> bool {
    matches!(
        item_contents(text.as_bytes(), Span::point(0)).0,
        TokenContents::Redirection(operator) if !operator.is_pipe()
    )
}

/// The assignment operator `text` spells, if any (nu's `is_assignment_operator`).
pub(crate) fn assignment_operator(text: &str) -> Option<AssignmentOperator> {
    assignment_operator_bytes(text.as_bytes())
}

/// The assignment operator of [`item_contents`], if `text` is one.
#[inline(always)]
fn assignment_operator_bytes(text: &[u8]) -> Option<AssignmentOperator> {
    Some(match text {
        b"=" => AssignmentOperator::Assign,
        b"+=" => AssignmentOperator::AddAssign,
        b"-=" => AssignmentOperator::SubtractAssign,
        b"*=" => AssignmentOperator::MultiplyAssign,
        b"/=" => AssignmentOperator::DivideAssign,
        b"++=" => AssignmentOperator::ConcatenateAssign,
        _ => return None,
    })
}

/// What a lexed item at `span` is: an assignment operator, a redirection, or a plain item (nu's
/// `lex_item` does this at its end). A bash-ism is the token nu makes of it, with its error.
fn item_contents(text: &[u8], span: Span) -> (TokenContents, Option<Box<LexError>>) {
    if let Some(operator) = assignment_operator_bytes(text) {
        return (TokenContents::AssignmentOperator(operator), None);
    }
    let kind = match text {
        b"out>" | b"o>" => TokenContents::Redirection(RedirectionOperator::Out),
        b"out>>" | b"o>>" => TokenContents::Redirection(RedirectionOperator::OutAppend),
        b"err>" | b"e>" => TokenContents::Redirection(RedirectionOperator::Err),
        b"err>>" | b"e>>" => TokenContents::Redirection(RedirectionOperator::ErrAppend),
        b"err>|" | b"e>|" => TokenContents::Redirection(RedirectionOperator::ErrPipe),
        b"out+err>" | b"err+out>" | b"o+e>" | b"e+o>" => {
            TokenContents::Redirection(RedirectionOperator::OutErr)
        }
        b"out+err>>" | b"err+out>>" | b"o+e>>" | b"e+o>>" => {
            TokenContents::Redirection(RedirectionOperator::OutErrAppend)
        }
        b"out+err>|" | b"err+out>|" | b"o+e>|" | b"e+o>|" => {
            TokenContents::Redirection(RedirectionOperator::OutErrPipe)
        }
        b"out>|" | b"o>|" => return (TokenContents::Pipe, Some(Box::new(LexError::OutPipe(span)))),
        b"&&" => {
            return (
                TokenContents::Pipe,
                Some(Box::new(LexError::ShellAndAnd(span))),
            );
        }
        b"2>" => {
            return (
                TokenContents::Redirection(RedirectionOperator::Err),
                Some(Box::new(LexError::ShellErrRedirect(span))),
            );
        }
        b"2>&1" => {
            return (
                TokenContents::Pipe,
                Some(Box::new(LexError::ShellOutErrRedirect(span))),
            );
        }
        _ => TokenContents::Item,
    };
    (kind, None)
}

/// A tiny helper for tests and debugging: lex and return `(kind, text)` pairs.
#[cfg(test)]
pub(crate) fn lex_debug(text: &str, options: LexOptions) -> Vec<(TokenContents, &str)> {
    lex(text, 0, options)
        .unwrap()
        .into_iter()
        .map(|t| (t.contents, t.text(text)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use TokenContents::*;

    #[test]
    fn simple_pipeline() {
        let tokens = lex_debug("ls -l | where size > 1kb\n", LexOptions::BLOCK);
        let kinds: Vec<_> = tokens.iter().map(|(k, t)| (*k, *t)).collect();
        assert_eq!(
            kinds,
            vec![
                (Item, "ls"),
                (Item, "-l"),
                (Pipe, "|"),
                (Item, "where"),
                (Item, "size"),
                (Item, ">"),
                (Item, "1kb"),
                (Eol, "\n"),
                (Eof, ""),
            ]
        );
    }

    #[test]
    fn brackets_are_one_item() {
        let tokens = lex_debug("echo [1 2 {a: (3 | 4)}] {|x| $x}", LexOptions::BLOCK);
        assert_eq!(tokens[1], (Item, "[1 2 {a: (3 | 4)}]"));
        assert_eq!(tokens[2], (Item, "{|x| $x}"));
    }

    #[test]
    fn strings_and_interpolation() {
        let tokens = lex_debug(
            r#"print "a | b" 'c;d' `e f` $"x (1 + ")") y" foo"bar""#,
            LexOptions::BLOCK,
        );
        let texts: Vec<_> = tokens.iter().map(|t| t.1).collect();
        assert_eq!(
            texts,
            vec![
                "print",
                "\"a | b\"",
                "'c;d'",
                "`e f`",
                "$\"x (1 + \")\") y\"",
                "foo\"bar\"",
                ""
            ]
        );
    }

    #[test]
    fn raw_strings() {
        let tokens = lex_debug("echo r#'a ' # b'# r##'c'#'##", LexOptions::BLOCK);
        assert_eq!(tokens[1], (Item, "r#'a ' # b'#"));
        assert_eq!(tokens[2], (Item, "r##'c'#'##"));
    }

    #[test]
    fn comments_and_pipe_continuation() {
        let src = "ls\n# c\n| length # trailing\n";
        let tokens = lex_debug(src, LexOptions::BLOCK);
        let kinds: Vec<_> = tokens.iter().map(|t| t.0).collect();
        assert_eq!(
            kinds,
            vec![Item, Eol, Comment, Eol, Pipe, Item, Comment, Eol, Eof]
        );
    }

    #[test]
    fn redirections_and_assignment() {
        let tokens = lex_debug("cmd o> f e>| x; $y += 1", LexOptions::BLOCK);
        let kinds: Vec<_> = tokens.iter().map(|t| t.0).collect();
        assert_eq!(
            kinds,
            vec![
                Item,
                Redirection(RedirectionOperator::Out),
                Item,
                Redirection(RedirectionOperator::ErrPipe),
                Item,
                Semicolon,
                Item,
                TokenContents::AssignmentOperator(super::AssignmentOperator::AddAssign),
                Item,
                Eof
            ]
        );
    }

    #[test]
    fn special_tokens_split() {
        let tokens = lex_debug("a:1, b: 2", LexOptions::RECORD_KEY);
        let texts: Vec<_> = tokens.iter().map(|t| t.1).collect();
        assert_eq!(texts, vec!["a", ":", "1", "b", ":", "2", ""]);
        let tokens = lex_debug("$x.a?.0", LexOptions::CELL_PATH);
        let texts: Vec<_> = tokens.iter().map(|t| t.1).collect();
        assert_eq!(texts, vec!["$x", ".", "a", "?", ".", "0", ""]);
    }

    #[test]
    fn signature_angle_brackets() {
        let tokens = lex_debug("x: list<record<a: int>>, y", LexOptions::SIGNATURE);
        let texts: Vec<_> = tokens.iter().map(|t| t.1).collect();
        assert_eq!(texts, vec!["x", ":", "list<record<a: int>>", ",", "y", ""]);
    }

    #[test]
    fn comments_inside_brackets_are_part_of_item() {
        let tokens = lex_debug("[\n  1 # one ]\n  2\n]", LexOptions::BLOCK);
        assert_eq!(tokens[0].0, Item);
        assert_eq!(tokens.len(), 2);
    }

    #[test]
    fn unclosed_errors() {
        let err = lex("echo [1 2", 0, LexOptions::BLOCK).unwrap_err();
        assert!(matches!(
            err.kind,
            ErrorKind::Unclosed { delimiter: "]", .. }
        ));
        let err = lex("echo 'abc", 0, LexOptions::BLOCK).unwrap_err();
        assert!(matches!(
            err.kind,
            ErrorKind::Unclosed { delimiter: "'", .. }
        ));
        let err = lex("echo )", 0, LexOptions::BLOCK).unwrap_err();
        assert!(matches!(err.kind, ErrorKind::Unbalanced { found: ")", .. }));
        let err = lex("a && b", 0, LexOptions::BLOCK).unwrap_err();
        assert!(matches!(
            err.kind,
            ErrorKind::ShellSyntax { found: "&&", .. }
        ));
    }

    #[test]
    fn absolute_offsets_with_base() {
        let tokens = lex("a b", 10, LexOptions::BLOCK).unwrap();
        assert_eq!(tokens[1].span, Span::new(12, 13));
        assert_eq!(tokens[2].span, Span::point(13));
    }

    #[test]
    fn prefix_lexing_stops_early() {
        let tokens = lex_n_tokens("a: 1, b: 2", 0, LexOptions::BRACE_PROBE, 2).unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[1].text("a: 1, b: 2"), ":");
    }

    /// The tokens nu's lexer makes of `text` and the errors it reports, which it goes on past.
    fn recovering(text: &str) -> (Vec<(TokenContents, &str)>, Vec<LexError>) {
        let lexed = lex_recovering(
            text,
            0,
            LexOptions::BLOCK,
            usize::MAX,
            &mut GroupEnds::default(),
        )
        .unwrap();
        let tokens = lexed
            .tokens
            .iter()
            .map(|token| (token.contents, token.text(text)))
            .collect();
        let errors = lexed.errors.into_iter().map(|(_, error)| error).collect();
        (tokens, errors)
    }

    /// Each item nu reports an error in ends where nu's does: just after a closer that closes
    /// nothing, at the end of the text when something is left open, before a backslash that
    /// ends a string, and after the `#`s of a raw string without its quote.
    #[test]
    fn errors_end_items_where_nu_does() {
        let (tokens, errors) = recovering("{a: {b: 5}}} x");
        assert_eq!(tokens[..2], [(Item, "{a: {b: 5}}}"), (Item, "x")]);
        assert!(matches!(
            errors[..],
            [LexError::Unbalanced {
                open: "{",
                close: "}",
                innermost: None,
                ..
            }]
        ));
        let (tokens, errors) = recovering("echo [1 2");
        assert_eq!(tokens[..2], [(Item, "echo"), (Item, "[1 2")]);
        assert!(matches!(
            errors[..],
            [LexError::Unclosed { delimiter: "]", open, end }]
                if open == Span::new(5, 6) && end == Span::new(8, 9)
        ));
        let (tokens, _) = recovering("\"say \\");
        assert_eq!(tokens[..2], [(Item, "\"say "), (Item, "\\")]);
        let (tokens, errors) = recovering("r#abc");
        assert_eq!(tokens[..2], [(Item, "r#"), (Item, "abc")]);
        assert!(matches!(errors[..], [LexError::RawStringQuote(span)] if span == Span::new(2, 3)));
    }

    /// A bash-ism is the token nu makes of it, and a `;` right after a `|` is reported: each
    /// error is kept, in order.
    #[test]
    fn bashisms_are_nus_tokens() {
        let (tokens, errors) = recovering("a && b 2> f; c | ; d");
        let kinds: Vec<TokenContents> = tokens.iter().map(|(kind, _)| *kind).collect();
        assert_eq!(
            kinds,
            [
                Item,
                Pipe,
                Item,
                Redirection(RedirectionOperator::Err),
                Item,
                Semicolon,
                Item,
                Pipe,
                Semicolon,
                Item,
                Eof
            ]
        );
        assert!(matches!(
            errors[..],
            [
                LexError::ShellAndAnd(_),
                LexError::ShellErrRedirect(_),
                LexError::SemicolonAfterPipe(_)
            ]
        ));
    }

    /// nu labels a missing `}` at a `|` starting a line inside it, when the line before it does
    /// not go on to it, and a missing `]` at the first `{` right inside it.
    #[test]
    fn unclosed_brackets_are_labeled_where_the_closer_belongs() {
        let (_, errors) = recovering("each {|x|\n  $x\n| length");
        assert!(matches!(
            errors[..],
            [LexError::Unclosed { delimiter: "}", open, end }]
                if open == Span::new(5, 6) && end == Span::new(15, 16)
        ));
        let (_, errors) = recovering("def f [\n  x {}\n y");
        assert!(matches!(
            errors[..],
            [LexError::Unclosed { delimiter: "]", open, end }]
                if open == Span::new(6, 7) && end == Span::new(12, 13)
        ));
    }
}
