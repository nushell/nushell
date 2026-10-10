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
//! The lexer reads a character stream ([`crate::input::Input`]): [`next_token`]
//! skips whitespace and dispatches on the next byte. An item is measured by
//! `item_length`, a byte loop like nu's `lex_item`, because an item's end
//! depends on the quotes and brackets open at each byte rather than on a
//! grammar; [`GroupEnds`] lets it jump over a bracket group it measured before.

use winnow::stream::{Location, Stream};

use crate::error::{Diagnostic, ErrorKind};
use crate::input::{Input, ParseFailure, ParseResult, cut, input, into_diagnostic, pos, span_from};
use crate::span::Span;

/// Which stream a redirection applies to and where it goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
            RedirectionOperator::OutAppend | RedirectionOperator::ErrAppend | RedirectionOperator::OutErrAppend
        )
    }

    /// `true` for `>|` variants, which redirect into the next pipeline element.
    pub fn is_pipe(self) -> bool {
        matches!(self, RedirectionOperator::ErrPipe | RedirectionOperator::OutErrPipe)
    }

    /// Which stream(s) are redirected.
    pub fn source(self) -> RedirectionSource {
        match self {
            RedirectionOperator::Out | RedirectionOperator::OutAppend => RedirectionSource::Stdout,
            RedirectionOperator::Err | RedirectionOperator::ErrAppend | RedirectionOperator::ErrPipe => {
                RedirectionSource::Stderr
            }
            RedirectionOperator::OutErr | RedirectionOperator::OutErrAppend | RedirectionOperator::OutErrPipe => {
                RedirectionSource::StdoutAndStderr
            }
        }
    }
}

/// The stream a redirection reads from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
        LexOptions { skip_comments, stops }
    }

    /// The options used for blocks and the top level of a file.
    pub const BLOCK: LexOptions = LexOptions::with_stops(&StopBytes::new(&[], &[], false), false);
    /// Options for subexpressions `( ... )`: newlines are whitespace, so a
    /// parenthesised pipeline may span several lines.
    pub const SUBEXPRESSION: LexOptions = LexOptions::with_stops(&StopBytes::new(b"\n\r", &[], false), false);
    /// Options for list interiors: commas and newlines are whitespace.
    pub const LIST: LexOptions = LexOptions::with_stops(&StopBytes::new(b"\n\r,", &[], false), false);
    /// Options for record interiors: like lists, and `:` is special.
    pub const RECORD_KEY: LexOptions = LexOptions::with_stops(&StopBytes::new(b"\n\r,", b":", false), false);
    /// Options for record values: like lists, but nothing is special.
    pub const RECORD_VALUE: LexOptions = LexOptions::with_stops(&StopBytes::new(b"\n\r,", &[], false), false);
    /// Options for signatures `[a: int, --flag(-f)]`.
    pub const SIGNATURE: LexOptions = LexOptions::with_stops(&StopBytes::new(b"\n\r", b":=,", true), false);
    /// Options for type parameters `list<int>`, `oneof<a, b>`, `record<a: int>`
    /// (nu's `lex_signature` in `parse_type_params`): `:` and `,` are special.
    /// nu skips comments here; the parser records them and drops them.
    pub const TYPE_PARAMS: LexOptions = LexOptions::with_stops(&StopBytes::new(b"\n\r", b":,", true), false);
    /// Options for the type of a declared variable `let x: record<a: int>`
    /// (nu's `lex_signature` in `parse_var_with_opt_type`): `<`/`>` pair up,
    /// so a stray `]` or `}` inside them is unbalanced.
    pub const VAR_TYPE: LexOptions = LexOptions::with_stops(&StopBytes::new(b"", b",", true), true);
    /// Options for input/output type lists `[int -> string, nothing -> nothing]`.
    pub const IO_TYPES: LexOptions = LexOptions::with_stops(&StopBytes::new(b"\n\r,", &[], true), false);
    /// Options for cell paths: `.`, `?` and `!` are special.
    pub const CELL_PATH: LexOptions = LexOptions::with_stops(&StopBytes::new(b"\n\r", b".?!", false), true);
    /// Options for match blocks: commas and newlines are whitespace. A `|` still lexes as a
    /// [`TokenContents::Pipe`], which separates the alternatives of an or-pattern (`1 | 2 => x`).
    pub const MATCH: LexOptions = LexOptions::with_stops(&StopBytes::new(b" \r\n,", &[], false), false);
    /// Options for the first two tokens of a `{...}` body, used to decide what it is.
    pub const BRACE_PROBE: LexOptions = LexOptions::with_stops(&StopBytes::new(b"\r\n\t", b":", false), true);
    /// Options for binary literals `0x[ff 00]`.
    pub const BINARY: LexOptions = LexOptions::with_stops(&StopBytes::new(b",\r\n", &[], false), false);
    /// Options for match list patterns.
    pub const PATTERN_LIST: LexOptions = LexOptions::with_stops(&StopBytes::new(b"\n\r,", &[], false), false);
    /// Options for record patterns.
    pub const PATTERN_RECORD: LexOptions = LexOptions::with_stops(&StopBytes::new(b"\n\r,", b":", false), false);
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
        Self { start: span.start, len: span.len(), ends: Vec::new() }
    }

    /// The group opening at `open` closes at `close`. A group opening outside the covered text,
    /// or closing too far from `start` for a `u32`, is not recorded; scans then measure it again.
    fn record(&mut self, open: usize, close: usize) {
        let Some(index) = open.checked_sub(self.start).filter(|&index| index < self.len) else { return };
        let Ok(end) = u32::try_from(close - self.start + 1) else { return };
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

/// nu's `ExtraTokens` for a `;` right after a `|`.
#[cold]
#[inline(never)]
fn semicolon_after_pipe(span: Span) -> Diagnostic {
    Diagnostic::new(ErrorKind::ExtraTokens, span).with_help("a `|` must be followed by a command before the `;`")
}

/// Like [`lex`], but stop after `max_tokens` tokens, not counting `Eof`
/// (nu's `lex_n_tokens`).
///
/// This is used to look at the first couple of tokens of a `{ ... }` body
/// without lexing all of it.
pub fn lex_n_tokens(text: &str, base: usize, options: LexOptions, max_tokens: usize) -> Result<Vec<Token>, Diagnostic> {
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
    let mut input = input(text, base);
    // An estimate of the token count (one per six bytes), never more than the budget and `Eof`.
    let capacity = (text.len() / 6).max(4).min(max_tokens.saturating_add(1));
    let mut tokens: Vec<Token> = Vec::with_capacity(capacity);
    // nu's `is_complete`: a `;` after a `|` with no item between them is
    // "extra tokens" (`ls |; ls`, and `{|x|; 1}`, whose parameter list ends
    // with a plain `|`). Newlines, comments and `||` leave it as it is.
    let mut after_pipe = false;
    while tokens.len() < max_tokens {
        match next_token(&mut input, options, groups) {
            Ok(Some(token)) => {
                match token.contents {
                    TokenContents::Pipe => after_pipe = true,
                    TokenContents::Semicolon if after_pipe => return Err(semicolon_after_pipe(token.span)),
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
    tokens.push(Token { contents: TokenContents::Eof, span: Span::point(pos(&input)) });
    Ok(tokens)
}

/// The next token of `input` after whitespace, or `None` at its end. With a
/// preset that skips comments (`VAR_TYPE`, `CELL_PATH`, ...) a comment is
/// passed over. Bracket groups are measured with `groups` (see [`GroupEnds`]).
#[inline]
pub fn next_token(input: &mut Input<'_>, options: LexOptions, groups: &mut GroupEnds) -> ParseResult<Option<Token>> {
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

/// One token, dispatched on its first byte. `None` for input that produces no
/// token (a skipped comment).
fn lex_token(input: &mut Input<'_>, options: LexOptions, groups: &mut GroupEnds) -> ParseResult<Option<Token>> {
    let start = pos(input);
    let text: &str = input.input.as_ref();
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
        _ => Some(lex_item(input, options, groups)?),
    };
    Ok(kind.map(|kind| Token { contents: kind, span: span_from(input, start) }))
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
    /// `<`, paired only in a signature (`StopBytes::in_signature`).
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
    matches!(text, b"o>" | b"out>" | b"e>" | b"err>" | b"o+e>" | b"e+o>" | b"out+err>" | b"err+out>")
}

/// Lex one item at the current position (nu's `lex_item`): [`item_length`]
/// measures it, [`item_contents`] tells an operator spelling from a plain item,
/// and the stream advances past it.
fn lex_item(input: &mut Input<'_>, options: LexOptions, groups: &mut GroupEnds) -> ParseResult<TokenContents> {
    let text = *input.input.as_ref();
    let bytes = text.as_bytes();
    let base = input.state.0 + input.current_token_start();
    let offset = item_length(bytes, base, options, false, groups)?;
    // An empty item would leave the stream where it is, and `lex_n_tokens_with` would loop.
    if offset == 0 {
        return Err(cut(Diagnostic::new(ErrorKind::UnexpectedEof("command"), Span::point(base))));
    }
    let item_text = &bytes[..offset];
    let kind = item_contents(item_text).map_err(|(kind, help)| {
        let mut d = Diagnostic::new(kind, Span::new(base, base + offset));
        if let Some(h) = help {
            d = d.with_help(h);
        }
        cut(d)
    })?;
    input.next_slice(offset);
    Ok(kind)
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
    item_length(text.as_bytes(), base, LexOptions::BLOCK, true, groups).ok().map(|offset| offset - 1)
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
) -> ParseResult<usize> {
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
    let in_signature = stops.in_signature;

    // nu's `is_item_terminator`: only at bracket depth zero does a terminator end the item.
    let is_terminator =
        |brackets: &[(Bracket, usize)], byte: u8| brackets.is_empty() && stops.terminators.contains(byte);

    while offset < bytes.len() {
        // Outside quotes most bytes (letters, digits, `$`, `-`, ...) are ones the match below
        // passes over, so skip the whole run in this tight loop: it runs for every byte of every
        // item. `StopBytes::item` lists the bytes that matter at depth zero, `group` those inside
        // brackets.
        if quote.is_none() {
            let stop = if brackets.is_empty() { &stops.item } else { &stops.group };
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
                let escaped =
                    interp_subexpr_step(&mut interp_level, byte, absolute(offset)) && offset + 1 < bytes.len();
                offset += if escaped { 2 } else { 1 };
                previous = Some(byte);
                continue;
            }
            Some((open_quote, open_at)) => match byte {
                b'\\' if open_quote == b'"' => {
                    if offset + 1 >= bytes.len() {
                        return Err(unclosed_error(quote_str(open_quote), open_at, absolute(offset + 1)));
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
                    in_comment =
                        previous.is_none_or(|previous| previous.is_ascii() && char::from(previous).is_whitespace())
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
                    if let Some(open) = close_bracket(&mut brackets, byte, absolute(offset))?
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

    if let Some(&(closer, open_at)) = interp_level.first() {
        return Err(unclosed_error(quote_str(closer), open_at, absolute(offset)));
    }
    if let Some((quote, open_at)) = quote {
        return Err(unclosed_error(quote_str(quote), open_at, absolute(offset)));
    }
    if let Some(&(open, open_at)) = brackets.last() {
        return Err(unclosed_error(open.closer(), open_at, absolute(offset)));
    }
    Ok(offset)
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
    /// Whether `<` and `>` nest as brackets (type annotations such as `list<int>`). Such a scan
    /// neither records nor jumps over bracket groups (see [`GroupEnds`]).
    in_signature: bool,
}

impl StopBytes {
    /// The sets for one [`LexOptions`] preset:
    /// - `additional_whitespace`: extra bytes treated as whitespace (in addition to space, tab,
    ///   `\r`); including `\n` suppresses [`TokenContents::Eol`] tokens.
    /// - `special_tokens`: bytes that are emitted as single-character items when they start a
    ///   token and that terminate the item otherwise (e.g. `:` in records).
    /// - `in_signature`: whether `<` and `>` nest as brackets.
    const fn new(additional_whitespace: &[u8], special_tokens: &[u8], in_signature: bool) -> Self {
        let brackets: &[u8] = if in_signature { b"[]{}()<>" } else { b"[]{}()" };
        let terminators = ByteSet::EMPTY.with(b" \t\n\r|;").with(additional_whitespace).with(special_tokens);
        StopBytes {
            item: terminators.with(b"#'\"`").with(brackets),
            group: ByteSet::EMPTY.with(b"\n\r#'\"`").with(brackets),
            terminators,
            special_tokens: ByteSet::EMPTY.with(special_tokens),
            whitespace: ByteSet::EMPTY.with(b" \t\r").with(additional_whitespace),
            in_signature,
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

/// The cut error for a delimiter opened at `open_at` and still open where the text ends, at
/// `at`; `delimiter` is the closer it needs.
fn unclosed_error(delimiter: &'static str, open_at: usize, at: usize) -> winnow::error::ErrMode<ParseFailure> {
    cut(Diagnostic::new(ErrorKind::Unclosed { delimiter, open: Span::new(open_at, open_at + 1) }, Span::point(at)))
}

/// Pop the bracket closed by `closer`, returning where it opened, or report the
/// mismatch. With no bracket open, a `]` is ordinary text (`a]`) and gives `None`,
/// while a `}` or `)` is an error; a closer that does not match the innermost open
/// bracket (`(a]`) is always an error.
fn close_bracket(brackets: &mut Vec<(Bracket, usize)>, closer: u8, at: usize) -> ParseResult<Option<usize>> {
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
        Some(&(open, open_at)) => Err(unbalanced_error(found, open, open_at, at)),
        None if closer == b']' => Ok(None),
        None => Err(cut(Diagnostic::new(
            ErrorKind::Unbalanced { found, expected: expected.opener() },
            Span::new(at, at + 1),
        ))),
    }
}

/// The cut error for a closer `found` at `at` that does not match the innermost open bracket,
/// `open` at `open_at`; the help names that bracket.
fn unbalanced_error(
    found: &'static str,
    open: Bracket,
    open_at: usize,
    at: usize,
) -> winnow::error::ErrMode<ParseFailure> {
    cut(Diagnostic::new(ErrorKind::Unbalanced { found, expected: open.opener() }, Span::new(at, at + 1))
        .with_help(format!("the innermost open delimiter is `{}` at byte {open_at}", open.opener())))
}

/// Scan a raw string `r#'...'#` starting at the `r` (nu's `lex_raw_string`);
/// returns the offset just past it. The string ends at the first `'` followed by
/// as many `#`s as follow the `r`, so `r##'a'#'##` holds `a'#`. Like nu, that `'`
/// may be the opening quote itself: the raw part of `r#'#a'#` is `r#'#`, and the
/// `a'#` after it opens a quote that never closes.
fn lex_raw_string(bytes: &[u8], start: usize, absolute: impl Fn(usize) -> usize) -> ParseResult<usize> {
    let mut hashes = 0;
    while bytes.get(start + 1 + hashes) == Some(&b'#') {
        hashes += 1;
    }
    let quote_at = start + 1 + hashes;
    if bytes.get(quote_at) != Some(&b'\'') {
        return Err(cut(Diagnostic::expected("`'` after `r#`", Span::point(absolute(quote_at)))));
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
    Err(cut(Diagnostic::new(
        ErrorKind::Unclosed { delimiter: "'", open: Span::new(absolute(start), absolute(quote_at + 1)) },
        Span::point(absolute(bytes.len())),
    )))
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

/// What a lexed item is: an assignment operator, a redirection, or a plain
/// item (nu's `lex_item` does this at its end). Bash-isms are refused.
fn item_contents(text: &[u8]) -> Result<TokenContents, (ErrorKind, Option<&'static str>)> {
    if let Some(operator) = assignment_operator_bytes(text) {
        return Ok(TokenContents::AssignmentOperator(operator));
    }
    Ok(match text {
        b"out>" | b"o>" => TokenContents::Redirection(RedirectionOperator::Out),
        b"out>>" | b"o>>" => TokenContents::Redirection(RedirectionOperator::OutAppend),
        b"err>" | b"e>" => TokenContents::Redirection(RedirectionOperator::Err),
        b"err>>" | b"e>>" => TokenContents::Redirection(RedirectionOperator::ErrAppend),
        b"err>|" | b"e>|" => TokenContents::Redirection(RedirectionOperator::ErrPipe),
        b"out+err>" | b"err+out>" | b"o+e>" | b"e+o>" => TokenContents::Redirection(RedirectionOperator::OutErr),
        b"out+err>>" | b"err+out>>" | b"o+e>>" | b"e+o>>" => {
            TokenContents::Redirection(RedirectionOperator::OutErrAppend)
        }
        b"out+err>|" | b"err+out>|" | b"o+e>|" | b"e+o>|" => {
            TokenContents::Redirection(RedirectionOperator::OutErrPipe)
        }
        b"out>|" | b"o>|" => {
            return Err((
                ErrorKind::ShellSyntax { found: "o>|", use_instead: "|" },
                Some("redirecting stdout to a pipe is the same as normal piping"),
            ));
        }
        b"&&" => {
            return Err((
                ErrorKind::ShellSyntax { found: "&&", use_instead: ";" },
                Some("use `;` to run commands in sequence, or `and` for boolean logic"),
            ));
        }
        b"2>" => return Err((ErrorKind::ShellSyntax { found: "2>", use_instead: "e>" }, None)),
        b"2>&1" => return Err((ErrorKind::ShellSyntax { found: "2>&1", use_instead: "o+e>" }, None)),
        _ => TokenContents::Item,
    })
}

/// A tiny helper for tests and debugging: lex and return `(kind, text)` pairs.
#[cfg(test)]
pub(crate) fn lex_debug(text: &str, options: LexOptions) -> Vec<(TokenContents, &str)> {
    lex(text, 0, options).unwrap().into_iter().map(|t| (t.contents, t.text(text))).collect()
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
        let tokens = lex_debug(r#"print "a | b" 'c;d' `e f` $"x (1 + ")") y" foo"bar""#, LexOptions::BLOCK);
        let texts: Vec<_> = tokens.iter().map(|t| t.1).collect();
        assert_eq!(texts, vec!["print", "\"a | b\"", "'c;d'", "`e f`", "$\"x (1 + \")\") y\"", "foo\"bar\"", ""]);
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
        assert_eq!(kinds, vec![Item, Eol, Comment, Eol, Pipe, Item, Comment, Eol, Eof]);
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
        assert!(matches!(err.kind, ErrorKind::Unclosed { delimiter: "]", .. }));
        let err = lex("echo 'abc", 0, LexOptions::BLOCK).unwrap_err();
        assert!(matches!(err.kind, ErrorKind::Unclosed { delimiter: "'", .. }));
        let err = lex("echo )", 0, LexOptions::BLOCK).unwrap_err();
        assert!(matches!(err.kind, ErrorKind::Unbalanced { found: ")", .. }));
        let err = lex("a && b", 0, LexOptions::BLOCK).unwrap_err();
        assert!(matches!(err.kind, ErrorKind::ShellSyntax { found: "&&", .. }));
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
}
