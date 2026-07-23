use nu_protocol::{ParseError, Span};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TokenContents {
    Item,
    Comment,
    Pipe,
    PipePipe,
    AssignmentOperator,
    ErrGreaterPipe,
    OutErrGreaterPipe,
    Semicolon,
    OutGreaterThan,
    OutGreaterGreaterThan,
    ErrGreaterThan,
    ErrGreaterGreaterThan,
    OutErrGreaterThan,
    OutErrGreaterGreaterThan,
    Eol,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Token {
    pub contents: TokenContents,
    pub span: Span,
}

impl Token {
    pub fn new(contents: TokenContents, span: Span) -> Token {
        Token { contents, span }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BlockKind {
    Paren,
    CurlyBracket,
    SquareBracket,
    AngleBracket,
}

/// An open delimiter on the lexer's nesting stack (kind + opener span only).
///
/// Opener spans are used only to *label* a real unclosed/unbalanced error for
/// miette. Indent/structure heuristics must never invent a parse failure — the
/// stack alone decides whether lexing failed.
#[derive(Clone, Copy, Debug)]
struct OpenFrame {
    kind: BlockKind,
    open_span: Span,
}

// A baseline token is terminated if it's not nested inside of a paired
// delimiter and the next character is one of: `|`, `;` or any
// whitespace.
fn is_item_terminator(
    block_level: &[OpenFrame],
    c: u8,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
) -> bool {
    block_level.is_empty()
        && (c == b' '
            || c == b'\t'
            || c == b'\n'
            || c == b'\r'
            || c == b'|'
            || c == b';'
            || additional_whitespace.contains(&c)
            || special_tokens.contains(&c))
}

/// Assignment operators have special handling distinct from math expressions, as they cause the
/// rest of the pipeline to be consumed.
pub fn is_assignment_operator(bytes: &[u8]) -> bool {
    matches!(bytes, b"=" | b"+=" | b"++=" | b"-=" | b"*=" | b"/=")
}

// A special token is one that is a byte that stands alone as its own token. For example
// when parsing a signature you may want to have `:` be able to separate tokens and also
// to be handled as its own token to notify you you're about to parse a type in the example
// `foo:bar`
fn is_special_item(block_level: &[OpenFrame], c: u8, special_tokens: &[u8]) -> bool {
    block_level.is_empty() && special_tokens.contains(&c)
}

fn closing_delimiter_str(block: BlockKind) -> &'static str {
    match block {
        BlockKind::Paren => ")",
        BlockKind::SquareBracket => "]",
        BlockKind::CurlyBracket => "}",
        BlockKind::AngleBracket => ">",
    }
}

fn opening_delimiter_str(block: BlockKind) -> &'static str {
    match block {
        BlockKind::Paren => "(",
        BlockKind::SquareBracket => "[",
        BlockKind::CurlyBracket => "{",
        BlockKind::AngleBracket => "<",
    }
}

/// Unexpected closer: report based on a *real* stack failure only.
///
/// Presentation may be reshaped when the stack failure is almost certainly due
/// to a missing opener nearby (e.g. missing `{` after `if`, or missing `[`
/// before list elements). That never invents a failure — only the stack decides
/// that lexing failed.
fn unbalanced_closer(
    closer: &'static str,
    default_open: &'static str,
    block_level: &[OpenFrame],
    close_span: Span,
    input: &[u8],
    span_offset: usize,
    token_start: usize,
) -> ParseError {
    if closer == "}"
        && block_level.is_empty()
        && let Some((kind, span)) =
            find_missing_open_brace_above(input, span_offset, token_start, close_span)
    {
        return match kind {
            MissingOpenBraceKind::ControlFlow => ParseError::LabeledErrorWithHelp {
                error: "Missing `{` to open a block".into(),
                label: "expected `{` after this condition".into(),
                help: "Add `{` after the `if` / `else if` / `while` / `for` / `try` / `match` condition. \
                       Without it, a later `}` has nothing to close."
                    .into(),
                span,
            },
            MissingOpenBraceKind::Record => ParseError::LabeledErrorWithHelp {
                error: "Missing `{` to open a record".into(),
                label: "expected `{` here".into(),
                help: "Record fields look like `{ key: value }`. \
                       Without the opening `{`, a later `}` has nothing to close."
                    .into(),
                span,
            },
        };
    }

    // Unexpected `]` with no open `[` on the stack (often inside a `{` block).
    // If this line looks like list elements ending in `]` without a `[`, point
    // at where `[` should start — e.g. `2 ($in_ten - 2) 0]` → missing `[`.
    // Do not apply when the top open is `(…` (wrong closer for a group).
    if closer == "]"
        && !matches!(
            block_level.last().map(|f| f.kind),
            Some(BlockKind::SquareBracket | BlockKind::Paren)
        )
        && let Some(open_span) = find_missing_list_open_bracket(input, span_offset, close_span)
    {
        return ParseError::LabeledErrorWithHelp {
            error: "Missing `[` to open a list".into(),
            label: "expected `[` here".into(),
            help: "This line ends with `]` but has no matching `[`. \
                   Add `[` before the list elements (e.g. `[2 ($in_ten - 2) 0]`)."
                .into(),
            span: open_span,
        };
    }

    // Unexpected `)` with no open `(` on the stack — e.g. `print -n ansi green)`
    // forgot the `(` before `ansi`. Point at where `(` should be inserted.
    // Do not apply when the top open is `[…` (wrong closer for a list, e.g. `[1, 2, 3)`).
    if closer == ")"
        && !matches!(
            block_level.last().map(|f| f.kind),
            Some(BlockKind::Paren | BlockKind::SquareBracket)
        )
        && let Some(open_span) = find_missing_open_paren(input, span_offset, close_span)
    {
        return ParseError::LabeledErrorWithHelp {
            error: "Missing `(` to open a group".into(),
            label: "expected `(` here".into(),
            help: "This expression ends with `)` but has no matching `(`. \
                   Add `(` before the grouped expression (e.g. `(ansi green)`)."
                .into(),
            span: open_span,
        };
    }

    // Stack top is the unmatched open (if any); otherwise the expected opener
    // for this closer. (Missing-`[` and missing-`{` cases are handled above.)
    let open = block_level
        .last()
        .map(|f| opening_delimiter_str(f.kind))
        .unwrap_or(default_open);
    ParseError::unbalanced(open, closer, close_span)
}

/// On a line ending with an unexpected `]`, if there is no `[` before it on
/// that line, the human almost always forgot the opening bracket.
///
/// Returns a span at the start of the list content (after indent).
fn find_missing_list_open_bracket(
    input: &[u8],
    span_offset: usize,
    close_span: Span,
) -> Option<Span> {
    let close_local = close_span.start.checked_sub(span_offset)?;
    if close_local > input.len() {
        return None;
    }
    let line_start = input[..close_local]
        .iter()
        .rposition(|&b| b == b'\n' || b == b'\r')
        .map(|i| i + 1)
        .unwrap_or(0);
    let line = &input[line_start..close_local];
    // No `[` on this line before the `]` → missing open.
    if line.contains(&b'[') {
        return None;
    }
    // Skip pure-whitespace lines or lines that are only the closer.
    let content_start = line
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(line.len());
    if content_start >= line.len() {
        return None;
    }
    // Avoid flagging things that clearly aren't lists (e.g. bare identifiers with ]).
    // Require some list-ish content: digits, `(`, `$`, `"`, `'`, or space-separated values.
    let content = &line[content_start..];
    let looks_like_list_elems = content.iter().any(|b| {
        b.is_ascii_digit() || matches!(*b, b'(' | b'$' | b'"' | b'\'' | b'`' | b'-' | b'.' | b' ')
    });
    if !looks_like_list_elems {
        return None;
    }
    let abs = span_offset + line_start + content_start;
    Some(Span::new(abs, abs + 1))
}

/// On a line ending with an unexpected `)`, if there is no `(` before it on
/// that line, the human almost always forgot the opening paren.
///
/// Returns a span where `(` should be inserted — preferably before the
/// expression being closed (e.g. before `ansi` in `print -n ansi green)`).
///
/// If the line already contains `(`, do not reshape: balanced groups with an
/// extra `)` (e.g. `print (ansi green))`) must stay plain `Unbalanced`.
fn find_missing_open_paren(input: &[u8], span_offset: usize, close_span: Span) -> Option<Span> {
    let close_local = close_span.start.checked_sub(span_offset)?;
    if close_local > input.len() {
        return None;
    }
    let line_start = input[..close_local]
        .iter()
        .rposition(|&b| b == b'\n' || b == b'\r')
        .map(|i| i + 1)
        .unwrap_or(0);
    let line = &input[line_start..close_local];

    // Any `(` on this line before the `)` means this is not a simple missing open
    // (balanced group + extra closer, nested mismatch, etc.).
    if line.contains(&b'(') {
        return None;
    }

    // Prefer the start of the last "argument group" after a command + flags,
    // so `print -n ansi green)` points at `ansi`, not `print`.
    if let Some(rel) = start_of_trailing_expr_group(line) {
        let abs = span_offset + line_start + rel;
        return Some(Span::new(abs, abs + 1));
    }

    let content_start = line
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(line.len());
    if content_start >= line.len() {
        return None;
    }
    let abs = span_offset + line_start + content_start;
    Some(Span::new(abs, abs + 1))
}

/// Byte offset into `line` of the trailing expression that should be wrapped in
/// `(…)`, skipping a leading command and short flags (`-n`, `--long`).
fn start_of_trailing_expr_group(line: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i < line.len() && line[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= line.len() {
        return None;
    }

    // First token: command name (skip it when there is more after flags).
    let first = i;
    while i < line.len() && !line[i].is_ascii_whitespace() {
        i += 1;
    }
    // Skip flags
    loop {
        while i < line.len() && line[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < line.len() && line[i] == b'-' {
            while i < line.len() && !line[i].is_ascii_whitespace() {
                i += 1;
            }
            continue;
        }
        break;
    }
    while i < line.len() && line[i].is_ascii_whitespace() {
        i += 1;
    }
    if i < line.len() {
        // Remaining content is the grouped expression (e.g. `ansi green`).
        Some(i)
    } else {
        // Only a command (or command + flags) — group from the first token.
        Some(first)
    }
}

/// Why a nearby line suggests a missing `{` when an unexpected `}` is found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MissingOpenBraceKind {
    /// `if` / `while` / `for` / `try` / `match` / `else` without a following `{`.
    ControlFlow,
    /// Record fields written without the opening `{` (e.g. `type: $x}`).
    Record,
}

/// Look above an unexpected `}` for a missing opening `{`.
///
/// Two common shapes (closest match wins):
///
/// Control-flow without a block:
/// ```text
///     if ($r.value | describe) != string
///       $r
///     }
/// ```
///
/// Bare record fields (missing `{` before the first `key:`):
/// ```text
///     | each {|lst|
///         type: $lst.0}
/// ```
///
/// Searches a window of source *before* the closer (not only the current lex
/// token), because an orphan `}` is often its own token after a balanced block.
fn find_missing_open_brace_above(
    input: &[u8],
    span_offset: usize,
    _token_start: usize,
    close_span: Span,
) -> Option<(MissingOpenBraceKind, Span)> {
    let close_local = close_span.start.checked_sub(span_offset)?;
    if close_local == 0 || close_local > input.len() {
        return None;
    }
    // ~2kB lookback is enough for typical "if … / body / }" and bare-record mistakes.
    let search_from = close_local.saturating_sub(2048);
    let region = &input[search_from..close_local];

    // Walk lines from nearest above the `}` upward; return the closest match.
    let mut line_end = region.len();
    while line_end > 0 {
        let line_start = region[..line_end]
            .iter()
            .rposition(|&b| b == b'\n' || b == b'\r')
            .map(|i| i + 1)
            .unwrap_or(0);
        let line = &region[line_start..line_end];
        // Strip trailing comment for the "has `{`?" check.
        let code = match line.iter().position(|&b| b == b'#') {
            Some(i) => &line[..i],
            None => line,
        };
        let trimmed = trim_ascii_start(code);
        if !trimmed.is_empty() {
            if line_looks_like_control_flow_without_brace(trimmed) {
                // Point at the last non-whitespace of the condition (where `{` belongs).
                if let Some(span) = span_at_line_end(code, span_offset, search_from, line_start) {
                    return Some((MissingOpenBraceKind::ControlFlow, span));
                }
            } else if line_looks_like_bare_record_fields(trimmed) {
                // Point at the first field key (where `{` should be inserted).
                let key_rel = code
                    .iter()
                    .position(|b| !b.is_ascii_whitespace())
                    .unwrap_or(0);
                let abs = span_offset + search_from + line_start + key_rel;
                return Some((MissingOpenBraceKind::Record, Span::new(abs, abs + 1)));
            }
        }
        if line_start == 0 {
            break;
        }
        // Move to previous line (skip the newline).
        line_end = line_start.saturating_sub(1);
        while line_end > 0 && matches!(region[line_end - 1], b'\n' | b'\r') {
            line_end -= 1;
        }
    }
    None
}

fn span_at_line_end(
    code: &[u8],
    span_offset: usize,
    search_from: usize,
    line_start: usize,
) -> Option<Span> {
    let mut trim_end = code.len();
    while trim_end > 0 && code[trim_end - 1].is_ascii_whitespace() {
        trim_end -= 1;
    }
    if trim_end == 0 {
        return None;
    }
    let abs_end = span_offset + search_from + line_start + trim_end;
    let abs_start = abs_end - 1;
    Some(Span::new(abs_start, abs_end))
}

fn trim_ascii_start(bytes: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    &bytes[i..]
}

fn trim_ascii_end(bytes: &[u8]) -> &[u8] {
    let mut end = bytes.len();
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    &bytes[..end]
}

fn line_looks_like_control_flow_without_brace(trimmed_line: &[u8]) -> bool {
    // Must not already open a block on this line.
    if trimmed_line.contains(&b'{') {
        return false;
    }
    // Keyword forms require a boundary so identifiers like `try_this` / `trying`
    // are not treated as bare `try`.
    let keywords: &[&[u8]] = &[
        b"else if ",
        b"else if\t",
        b"if ",
        b"if\t",
        b"if(",
        b"while ",
        b"while\t",
        b"while(",
        b"for ",
        b"for\t",
        b"match ",
        b"match\t",
        b"match(",
    ];
    let is_kw = keywords.iter().any(|kw| trimmed_line.starts_with(kw))
        || is_keyword_with_boundary(trimmed_line, b"try")
        || is_keyword_with_boundary(trimmed_line, b"else");
    // Bare `else` / `try` without `{` on the same line is incomplete for a block form.
    is_kw
}

/// True when `line` is exactly `keyword`, or `keyword` followed by a non-identifier
/// boundary (whitespace, `#`, etc.). Prevents `try_this` matching `try`.
fn is_keyword_with_boundary(line: &[u8], keyword: &[u8]) -> bool {
    if !line.starts_with(keyword) {
        return false;
    }
    match line.get(keyword.len()) {
        None => true,
        Some(b) => !is_ascii_ident_continue(*b),
    }
}

fn is_ascii_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// `type: $lst.0}` or similar — record fields without an opening `{` on the line.
///
/// Strong signal: ends with `}` and has `key: value` with no `{` anywhere on the
/// line. That `}` usually closed an outer block early; a later `}` then looks
/// unbalanced far below the real mistake.
fn line_looks_like_bare_record_fields(trimmed_line: &[u8]) -> bool {
    if trimmed_line.is_empty() || trimmed_line[0] == b'{' {
        return false;
    }
    // Need a trailing `}` on this line with no `{` at all (orphan closer).
    let line = trim_ascii_end(trimmed_line);
    if !line.ends_with(b"}") || line.contains(&b'{') {
        return false;
    }
    // Strip the trailing `}` for field shape checks.
    let mut body = &line[..line.len() - 1];
    body = trim_ascii_end(body);
    if body.is_empty() {
        return false;
    }

    let colon = match body.iter().position(|&b| b == b':') {
        Some(i) => i,
        None => return false,
    };
    let key = trim_ascii_end(&body[..colon]);
    let val = trim_ascii_start(&body[colon + 1..]);
    if key.is_empty() || val.is_empty() {
        return false;
    }
    // Signature lines like `def foo []: nothing -> table {` have `]:` — skip.
    if key.ends_with(b"]") {
        return false;
    }
    is_simple_record_key(key)
}

fn is_simple_record_key(key: &[u8]) -> bool {
    if key.is_empty() {
        return false;
    }
    if key[0] == b'"' {
        return key.len() >= 2 && key[key.len() - 1] == b'"';
    }
    // Bare identifier: letters/digits/_/- , starting with letter or `_`.
    let first = key[0];
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    key.iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn push_open(block_level: &mut Vec<OpenFrame>, kind: BlockKind, open_span: Span) {
    block_level.push(OpenFrame { kind, open_span });
}

fn quote_delimiter_str(quote: u8) -> &'static str {
    match quote {
        b'"' => "\"",
        b'\'' => "'",
        b'`' => "`",
        _ => "\"",
    }
}

/// Best-effort structure hint from bytes immediately before an opening delimiter.
/// Returns a short phrase like `record field ls` or `def foo`, or `None` if unsure.
pub fn delimiter_structure_hint(bytes_before_open: &[u8]) -> Option<String> {
    // Only inspect a small window to avoid pathological lookbacks.
    const WINDOW: usize = 80;
    let start = bytes_before_open.len().saturating_sub(WINDOW);
    let window = &bytes_before_open[start..];

    // Work on the last line only (high-confidence local context).
    let line = match window.iter().rposition(|&b| b == b'\n' || b == b'\r') {
        Some(i) => &window[i + 1..],
        None => window,
    };

    let trimmed = trim_ascii_end(line);
    if trimmed.is_empty() {
        return None;
    }

    // Record field: `ident:` with optional spaces before the opener we already excluded.
    if let Some(colon) = trimmed.iter().rposition(|&b| b == b':') {
        let before_colon = trim_ascii_end(&trimmed[..colon]);
        if let Some(name) = trailing_ident(before_colon) {
            return Some(format!("record field `{name}`"));
        }
    }

    // Keyword + optional name: `def foo`, `export def foo`, `module bar`, etc.
    let tokens = split_ascii_whitespace(trimmed);
    if tokens.is_empty() {
        return None;
    }

    let keywords = [
        "def", "module", "extern", "if", "match", "try", "for", "while", "loop", "export",
    ];

    // `export def name` / `export module name`
    if tokens[0] == "export" && tokens.len() >= 2 {
        let kw = tokens[1];
        if matches!(kw, "def" | "module" | "extern") {
            if let Some(name) = tokens.get(2).filter(|n| is_simple_ident(n.as_bytes())) {
                return Some(format!("`export {kw} {name}`"));
            }
            return Some(format!("`export {kw}`"));
        }
    }

    let kw = tokens[0];
    if keywords.contains(&kw) {
        if matches!(kw, "def" | "module" | "extern")
            && let Some(name) = tokens.get(1).filter(|n| is_simple_ident(n.as_bytes()))
        {
            return Some(format!("`{kw} {name}`"));
        }
        return Some(format!("`{kw}`"));
    }

    None
}

fn trailing_ident(bytes: &[u8]) -> Option<&str> {
    let mut end = bytes.len();
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    let mut start = end;
    while start > 0 {
        let b = bytes[start - 1];
        if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' {
            start -= 1;
        } else {
            break;
        }
    }
    if start == end {
        return None;
    }
    let ident = std::str::from_utf8(&bytes[start..end]).ok()?;
    if is_simple_ident(ident.as_bytes()) {
        Some(ident)
    } else {
        None
    }
}

fn is_simple_ident(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-')
}

fn split_ascii_whitespace(bytes: &[u8]) -> Vec<&str> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if let Ok(s) = std::str::from_utf8(&bytes[start..i]) {
            out.push(s);
        }
    }
    out
}

fn unclosed_from_open(
    input: &[u8],
    span_offset: usize,
    delimiter: &'static str,
    open_span: Span,
    end_span: Span,
) -> ParseError {
    let hint = open_span.start.checked_sub(span_offset).and_then(|local| {
        if local <= input.len() {
            delimiter_structure_hint(&input[..local])
        } else {
            None
        }
    });
    ParseError::unclosed_with_hint(delimiter, open_span, end_span, hint.as_deref())
}

/// A better place to put the "expected closer" miette label when the real stack
/// failure is only known at end-of-token (often far from the human mistake).
///
/// Never used to invent an error — only to choose spans for an error that
/// already exists because `block_level` is non-empty at the end.
#[derive(Clone, Copy, Debug)]
struct CloserLabelHint {
    /// Opener most likely related to the missing closer (often an inner `{|…`).
    open_span: Span,
    /// Where the missing closer probably belongs.
    expected_span: Span,
}

/// True if `c` can legally continue a multi-line construct onto the next line
/// (so a following line starting with `|` is not a missing-`}` signal).
fn continues_onto_next_line(c: u8) -> bool {
    matches!(
        c,
        b'|' | b'{' | b'(' | b'[' | b',' | b':' | b'+' | b'-' | b'*' | b'/' | b'=' | b'.'
    )
}

pub fn lex_item(
    input: &[u8],
    curr_offset: &mut usize,
    span_offset: usize,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    in_signature: bool,
) -> (Token, Option<ParseError>) {
    // Tracks the opening quote character and its span while inside a string.
    let mut quote_start: Option<(u8, Span)> = None;

    let mut in_comment = false;

    let token_start = *curr_offset;

    // Paired delimiters with opener spans (for labeling real unclosed errors only).
    let mut block_level: Vec<OpenFrame> = vec![];

    // Presentation-only: first place a missing `}` may belong (e.g. before a
    // pipeline step that should have been outside a closure). Used solely when
    // the stack still has openers at end-of-token — never to invent failures.
    let mut closer_label_hint: Option<CloserLabelHint> = None;

    // Line tracking for the presentation hint above (not for inventing errors).
    let mut at_line_start = true;
    // Last non-whitespace, non-comment char on the previous line (if any).
    let mut prev_line_continue = false;
    let mut last_sig_char: Option<u8> = None;

    // The process of slurping up a baseline token repeats:
    //
    // - String literal, which begins with `'` or `"`, and continues until
    //   the same character is encountered again.
    // - Delimiter pair, which begins with `[`, `(`, or `{`, and continues until
    //   the matching closing delimiter is found, skipping comments and string
    //   literals.
    // - When not nested inside of a delimiter pair, when a terminating
    //   character (whitespace, `|`, `;` or `#`) is encountered, the baseline
    //   token is done.
    // - Otherwise, accumulate the character into the current baseline token.
    //
    // Parse *failure* is decided only by the delimiter stack / quotes — never by
    // line-shape heuristics. Heuristics may only choose spans/help when a real
    // failure is reported.
    let mut previous_char = None;
    while let Some(c) = input.get(*curr_offset) {
        let c = *c;

        if let Some((start, open_span)) = quote_start {
            // Check if we're in an escape sequence
            if c == b'\\' && start == b'"' {
                // Go ahead and consume the escape character if possible
                if input.get(*curr_offset + 1).is_some() {
                    // Successfully escaped the character
                    *curr_offset += 2;
                    previous_char = Some(c);
                    at_line_start = false;
                    continue;
                } else {
                    let span = Span::new(span_offset + token_start, span_offset + *curr_offset);
                    let end_span = if span.end > span.start {
                        Span::new(span.end - 1, span.end)
                    } else {
                        span
                    };

                    return (
                        Token {
                            contents: TokenContents::Item,
                            span,
                        },
                        Some(unclosed_from_open(
                            input,
                            span_offset,
                            quote_delimiter_str(start),
                            open_span,
                            end_span,
                        )),
                    );
                }
            }
            // If we encountered the closing quote character for the current
            // string, we're done with the current string.
            if c == start {
                // Also need to check to make sure we aren't escaped
                quote_start = None;
            }
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b'#' && !in_comment {
            // To start a comment, It either need to be the first character of the token or prefixed with whitespace.
            in_comment = previous_char
                .map(char::from)
                .map(char::is_whitespace)
                .unwrap_or(true);
        } else if c == b'\n' || c == b'\r' {
            in_comment = false;
            if is_item_terminator(&block_level, c, additional_whitespace, special_tokens) {
                break;
            }
            // Commit previous line's trailing significant char for next-line `|` hints.
            // For `\r\n`, only commit/reset on `\n` so we don't double-reset.
            let is_newline_end = c == b'\n' || input.get(*curr_offset + 1) != Some(&b'\n');
            if is_newline_end {
                prev_line_continue = last_sig_char.is_some_and(continues_onto_next_line);
                at_line_start = true;
                last_sig_char = None;
            }
        } else if in_comment {
            if is_item_terminator(&block_level, c, additional_whitespace, special_tokens) {
                break;
            }
        } else if is_special_item(&block_level, c, special_tokens) && token_start == *curr_offset {
            *curr_offset += 1;
            break;
        } else if c == b'\'' || c == b'"' || c == b'`' {
            let open_span = Span::new(span_offset + *curr_offset, span_offset + *curr_offset + 1);
            quote_start = Some((c, open_span));
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b'[' {
            let open_span = Span::new(span_offset + *curr_offset, span_offset + *curr_offset + 1);
            push_open(&mut block_level, BlockKind::SquareBracket, open_span);
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b'<' && in_signature {
            let open_span = Span::new(span_offset + *curr_offset, span_offset + *curr_offset + 1);
            push_open(&mut block_level, BlockKind::AngleBracket, open_span);
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b'>' && in_signature {
            if let Some(OpenFrame {
                kind: BlockKind::AngleBracket,
                ..
            }) = block_level.last()
            {
                let _ = block_level.pop();
            }
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b']' {
            // Closing `]` — pop matching `[`, else real mismatch if another opener is open.
            if let Some(OpenFrame {
                kind: BlockKind::SquareBracket,
                ..
            }) = block_level.last()
            {
                let _ = block_level.pop();
            } else if !block_level.is_empty() {
                *curr_offset += 1;
                let span = Span::new(span_offset + token_start, span_offset + *curr_offset);
                let close_span = Span::new(span.end - 1, span.end);
                return (
                    Token {
                        contents: TokenContents::Item,
                        span,
                    },
                    Some(unbalanced_closer(
                        "]",
                        "[",
                        &block_level,
                        close_span,
                        input,
                        span_offset,
                        token_start,
                    )),
                );
            }
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b'{' {
            // Presentation only: `def name [\n  param\n {` without `]` — the body
            // `{` is where `]` should have been. Record for labeling if the `[`
            // is still open at end-of-token (real stack failure).
            if closer_label_hint.is_none()
                && let Some(frame) = block_level.last()
                && matches!(frame.kind, BlockKind::SquareBracket)
            {
                closer_label_hint = Some(CloserLabelHint {
                    open_span: frame.open_span,
                    expected_span: Span::new(
                        span_offset + *curr_offset,
                        span_offset + *curr_offset + 1,
                    ),
                });
            }
            let open_span = Span::new(span_offset + *curr_offset, span_offset + *curr_offset + 1);
            push_open(&mut block_level, BlockKind::CurlyBracket, open_span);
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b'}' {
            // Closing `}` — pop matching `{`, else real mismatch against stack top.
            if let Some(OpenFrame {
                kind: BlockKind::CurlyBracket,
                ..
            }) = block_level.last()
            {
                let _ = block_level.pop();
            } else {
                *curr_offset += 1;
                let span = Span::new(span_offset + token_start, span_offset + *curr_offset);
                let close_span = Span::new(span.end - 1, span.end);
                return (
                    Token {
                        contents: TokenContents::Item,
                        span,
                    },
                    Some(unbalanced_closer(
                        "}",
                        "{",
                        &block_level,
                        close_span,
                        input,
                        span_offset,
                        token_start,
                    )),
                );
            }
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b'(' {
            let open_span = Span::new(span_offset + *curr_offset, span_offset + *curr_offset + 1);
            push_open(&mut block_level, BlockKind::Paren, open_span);
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b')' {
            // Closing `)` — pop matching `(`, else real mismatch against stack top.
            if let Some(OpenFrame {
                kind: BlockKind::Paren,
                ..
            }) = block_level.last()
            {
                let _ = block_level.pop();
            } else {
                *curr_offset += 1;
                let span = Span::new(span_offset + token_start, span_offset + *curr_offset);
                let close_span = Span::new(span.end - 1, span.end);
                return (
                    Token {
                        contents: TokenContents::Item,
                        span,
                    },
                    Some(unbalanced_closer(
                        ")",
                        "(",
                        &block_level,
                        close_span,
                        input,
                        span_offset,
                        token_start,
                    )),
                );
            }
            last_sig_char = Some(c);
            at_line_start = false;
        } else if c == b'r' && input.get(*curr_offset + 1) == Some(b'#').as_ref() {
            // already checked `r#` pattern, so it's a raw string.
            let lex_result = lex_raw_string(input, curr_offset, span_offset);
            let span = Span::new(span_offset + token_start, span_offset + *curr_offset);
            if let Err(e) = lex_result {
                return (
                    Token {
                        contents: TokenContents::Item,
                        span,
                    },
                    Some(e),
                );
            }
            last_sig_char = Some(b'#');
            at_line_start = false;
        } else if c == b'|' && is_redirection(&input[token_start..*curr_offset]) {
            // matches err>| etc.
            *curr_offset += 1;
            break;
        } else if is_item_terminator(&block_level, c, additional_whitespace, special_tokens) {
            break;
        } else if !c.is_ascii_whitespace() {
            // Presentation hint only: a new line starting with `|` while nested
            // in `{…}`, when the previous line did not end with a continue char,
            // often means a missing `}` before this pipeline step (e.g. forgot
            // to close `{|n| … }` before `| upsert …`).
            //
            // We only *record* this; an error is emitted only if the stack is
            // still non-empty at end-of-token.
            if c == b'|'
                && at_line_start
                && !prev_line_continue
                && closer_label_hint.is_none()
                && let Some(frame) = block_level
                    .iter()
                    .rev()
                    .find(|f| matches!(f.kind, BlockKind::CurlyBracket))
            {
                closer_label_hint = Some(CloserLabelHint {
                    open_span: frame.open_span,
                    expected_span: Span::new(
                        span_offset + *curr_offset,
                        span_offset + *curr_offset + 1,
                    ),
                });
            }
            last_sig_char = Some(c);
            at_line_start = false;
        } else if at_line_start && (c == b' ' || c == b'\t') {
            // stay at line start until real content
        } else {
            at_line_start = false;
        }

        *curr_offset += 1;
        previous_char = Some(c);
    }

    let span = Span::new(span_offset + token_start, span_offset + *curr_offset);
    let end_span = if span.end > span.start {
        Span::new(span.end - 1, span.end)
    } else {
        span
    };

    if let Some((delim, open_span)) = quote_start {
        // The non-lite parse trims quotes on both sides, so we add the expected quote so that
        // anyone wanting to consume this partial parse (e.g., completions) will be able to get
        // correct information from the non-lite parse.
        return (
            Token {
                contents: TokenContents::Item,
                span,
            },
            Some(unclosed_from_open(
                input,
                span_offset,
                quote_delimiter_str(delim),
                open_span,
                end_span,
            )),
        );
    }

    // Still-unclosed openers at end of token: real stack failure.
    // Prefer a recorded closer-label hint when it refers to the *same* open frame
    // still on the stack (presentation only — error already exists).
    if let Some(frame) = block_level.last() {
        let (label_open, label_end) = closer_label_hint
            .filter(|h| h.open_span == frame.open_span)
            .map(|h| (h.open_span, h.expected_span))
            .unwrap_or((frame.open_span, end_span));

        let cause = unclosed_from_open(
            input,
            span_offset,
            closing_delimiter_str(frame.kind),
            label_open,
            label_end,
        );

        return (
            Token {
                contents: TokenContents::Item,
                span,
            },
            Some(cause),
        );
    }

    // If we didn't accumulate any characters, it's an unexpected error.
    if *curr_offset - token_start == 0 {
        return (
            Token {
                contents: TokenContents::Item,
                span,
            },
            Some(ParseError::UnexpectedEof("command".to_string(), span)),
        );
    }

    let mut err = None;
    let output = match &input[(span.start - span_offset)..(span.end - span_offset)] {
        bytes if is_assignment_operator(bytes) => Token {
            contents: TokenContents::AssignmentOperator,
            span,
        },
        b"out>" | b"o>" => Token {
            contents: TokenContents::OutGreaterThan,
            span,
        },
        b"out>>" | b"o>>" => Token {
            contents: TokenContents::OutGreaterGreaterThan,
            span,
        },
        b"out>|" | b"o>|" => {
            err = Some(ParseError::Expected(
                "`|`.  Redirecting stdout to a pipe is the same as normal piping.",
                span,
            ));
            Token {
                // HACK: For more accurate parsing aligned with user intention
                contents: TokenContents::Pipe,
                span,
            }
        }
        b"err>" | b"e>" => Token {
            contents: TokenContents::ErrGreaterThan,
            span,
        },
        b"err>>" | b"e>>" => Token {
            contents: TokenContents::ErrGreaterGreaterThan,
            span,
        },
        b"err>|" | b"e>|" => Token {
            contents: TokenContents::ErrGreaterPipe,
            span,
        },
        b"out+err>" | b"err+out>" | b"o+e>" | b"e+o>" => Token {
            contents: TokenContents::OutErrGreaterThan,
            span,
        },
        b"out+err>>" | b"err+out>>" | b"o+e>>" | b"e+o>>" => Token {
            contents: TokenContents::OutErrGreaterGreaterThan,
            span,
        },
        b"out+err>|" | b"err+out>|" | b"o+e>|" | b"e+o>|" => Token {
            contents: TokenContents::OutErrGreaterPipe,
            span,
        },
        b"&&" => {
            err = Some(ParseError::ShellAndAnd(span));
            Token {
                // HACK: For more accurate parsing aligned with user intention
                contents: TokenContents::Pipe,
                span,
            }
        }
        b"2>" => {
            err = Some(ParseError::ShellErrRedirect(span));
            Token {
                // HACK: For more accurate parsing aligned with user intention
                contents: TokenContents::ErrGreaterThan,
                span,
            }
        }
        b"2>&1" => {
            err = Some(ParseError::ShellOutErrRedirect(span));
            Token {
                // HACK: For more accurate parsing aligned with user intention
                contents: TokenContents::Pipe,
                span,
            }
        }
        _ => Token {
            contents: TokenContents::Item,
            span,
        },
    };
    (output, err)
}

fn lex_raw_string(
    input: &[u8],
    curr_offset: &mut usize,
    span_offset: usize,
) -> Result<(), ParseError> {
    // A raw string literal looks like `echo r#'Look, I can use 'single quotes'!'#`
    // If the next character is `#` we're probably looking at a raw string literal
    // so we need to read all the text until we find a closing `#`. This raw string
    // can contain any character, including newlines and double quotes without needing
    // to escape them.
    //
    // A raw string can contain many `#` as prefix,
    // incase if there is a `'#` or `#'` in the string itself.
    // E.g: r##'I can use '#' in a raw string'##
    let mut prefix_sharp_cnt = 0;
    let start = *curr_offset;
    while let Some(b'#') = input.get(start + prefix_sharp_cnt + 1) {
        prefix_sharp_cnt += 1;
    }

    // curr_offset is the character `r`, we need to move forward and skip all `#`
    // characters.
    //
    // e.g: r###'<body>
    //      ^
    //      ^
    //   curr_offset
    *curr_offset += prefix_sharp_cnt + 1;
    // the next one should be a single quote.
    if input.get(*curr_offset) != Some(&b'\'') {
        return Err(ParseError::Expected(
            "'",
            Span::new(span_offset + *curr_offset, span_offset + *curr_offset + 1),
        ));
    }

    *curr_offset += 1;
    let mut matches = false;
    while let Some(ch) = input.get(*curr_offset) {
        // check for postfix '###
        if *ch == b'#' {
            let start_ch = input[*curr_offset - prefix_sharp_cnt];
            let postfix = &input[*curr_offset - prefix_sharp_cnt + 1..=*curr_offset];
            if start_ch == b'\'' && postfix.iter().all(|x| *x == b'#') {
                matches = true;
                break;
            }
        }
        *curr_offset += 1
    }
    if !matches {
        let mut expected = '\''.to_string();
        expected.push_str(&"#".repeat(prefix_sharp_cnt));
        return Err(ParseError::UnexpectedEof(
            expected,
            Span::new(span_offset + *curr_offset - 1, span_offset + *curr_offset),
        ));
    }
    Ok(())
}

pub fn lex_signature(
    input: &[u8],
    span_offset: usize,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    skip_comment: bool,
) -> (Vec<Token>, Option<ParseError>) {
    let mut state = LexState {
        input,
        output: Vec::new(),
        error: None,
        span_offset,
    };
    lex_internal(
        &mut state,
        additional_whitespace,
        special_tokens,
        skip_comment,
        true,
        None,
    );
    (state.output, state.error)
}

#[derive(Debug)]
pub struct LexState<'a> {
    pub input: &'a [u8],
    pub output: Vec<Token>,
    pub error: Option<ParseError>,
    pub span_offset: usize,
}

/// Lex until the output is `max_tokens` longer than before the call, or until the input is exhausted.
/// The return value indicates how many tokens the call added to / removed from the output.
///
/// The behaviour here is non-obvious when `additional_whitespace` doesn't include newline:
/// If you pass a `state` where the last token in the output is an Eol, this might *remove* tokens.
pub fn lex_n_tokens(
    state: &mut LexState,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    skip_comment: bool,
    max_tokens: usize,
) -> isize {
    let n_tokens = state.output.len();
    lex_internal(
        state,
        additional_whitespace,
        special_tokens,
        skip_comment,
        false,
        Some(max_tokens),
    );
    // If this lex_internal call reached the end of the input, there may now be fewer tokens
    // in the output than before.
    let tokens_n_diff = (state.output.len() as isize) - (n_tokens as isize);
    let next_offset = state.output.last().map(|token| token.span.end);
    if let Some(next_offset) = next_offset {
        state.input = &state.input[next_offset - state.span_offset..];
        state.span_offset = next_offset;
    }
    tokens_n_diff
}

pub fn lex(
    input: &[u8],
    span_offset: usize,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    skip_comment: bool,
) -> (Vec<Token>, Option<ParseError>) {
    let mut state = LexState {
        input,
        output: Vec::new(),
        error: None,
        span_offset,
    };
    lex_internal(
        &mut state,
        additional_whitespace,
        special_tokens,
        skip_comment,
        false,
        None,
    );
    (state.output, state.error)
}

fn lex_internal(
    state: &mut LexState,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    skip_comment: bool,
    // within signatures we want to treat `<` and `>` specially
    in_signature: bool,
    max_tokens: Option<usize>,
) {
    let initial_output_len = state.output.len();

    let mut curr_offset = 0;

    let mut is_complete = true;
    while let Some(c) = state.input.get(curr_offset) {
        if max_tokens
            .is_some_and(|max_tokens| state.output.len() >= initial_output_len + max_tokens)
        {
            break;
        }
        let c = *c;
        if c == b'|' {
            // If the next character is `|`, it's either `|` or `||`.
            let idx = curr_offset;
            let prev_idx = idx;
            curr_offset += 1;

            // If the next character is `|`, we're looking at a `||`.
            if let Some(c) = state.input.get(curr_offset)
                && *c == b'|'
            {
                let idx = curr_offset;
                curr_offset += 1;
                state.output.push(Token::new(
                    TokenContents::PipePipe,
                    Span::new(state.span_offset + prev_idx, state.span_offset + idx + 1),
                ));
                continue;
            }

            // Otherwise, it's just a regular `|` token.

            // Before we push, check to see if the previous character was a newline.
            // If so, then this is a continuation of the previous line
            if let Some(prev) = state.output.last_mut() {
                match prev.contents {
                    TokenContents::Eol => {
                        *prev = Token::new(
                            TokenContents::Pipe,
                            Span::new(state.span_offset + idx, state.span_offset + idx + 1),
                        );
                        // And this is a continuation of the previous line if previous line is a
                        // comment line (combined with EOL + Comment)
                        //
                        // Initially, the last one token is TokenContents::Pipe, we don't need to
                        // check it, so the beginning offset is 2.
                        let mut offset = 2;
                        while state.output.len() > offset {
                            let index = state.output.len() - offset;
                            if state.output[index].contents == TokenContents::Comment
                                && state.output[index - 1].contents == TokenContents::Eol
                            {
                                state.output.remove(index - 1);
                                offset += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    _ => {
                        state.output.push(Token::new(
                            TokenContents::Pipe,
                            Span::new(state.span_offset + idx, state.span_offset + idx + 1),
                        ));
                    }
                }
            } else {
                state.output.push(Token::new(
                    TokenContents::Pipe,
                    Span::new(state.span_offset + idx, state.span_offset + idx + 1),
                ));
            }

            is_complete = false;
        } else if c == b';' {
            // If the next character is a `;`, we're looking at a semicolon token.

            if !is_complete && state.error.is_none() {
                state.error = Some(ParseError::ExtraTokens(Span::new(
                    curr_offset,
                    curr_offset + 1,
                )));
            }
            let idx = curr_offset;
            curr_offset += 1;
            state.output.push(Token::new(
                TokenContents::Semicolon,
                Span::new(state.span_offset + idx, state.span_offset + idx + 1),
            ));
        } else if c == b'\r' {
            // Ignore a stand-alone carriage return
            curr_offset += 1;
        } else if c == b'\n' {
            // If the next character is a newline, we're looking at an EOL (end of line) token.
            let idx = curr_offset;
            curr_offset += 1;
            if !additional_whitespace.contains(&c) {
                state.output.push(Token::new(
                    TokenContents::Eol,
                    Span::new(state.span_offset + idx, state.span_offset + idx + 1),
                ));
            }
        } else if c == b'#' {
            // If the next character is `#`, we're at the beginning of a line
            // comment. The comment continues until the next newline.
            let mut start = curr_offset;

            while let Some(input) = state.input.get(curr_offset) {
                if *input == b'\n' {
                    if !skip_comment {
                        state.output.push(Token::new(
                            TokenContents::Comment,
                            Span::new(state.span_offset + start, state.span_offset + curr_offset),
                        ));
                    }
                    start = curr_offset;

                    break;
                } else {
                    curr_offset += 1;
                }
            }
            if start != curr_offset && !skip_comment {
                state.output.push(Token::new(
                    TokenContents::Comment,
                    Span::new(state.span_offset + start, state.span_offset + curr_offset),
                ));
            }
        } else if c == b' ' || c == b'\t' || additional_whitespace.contains(&c) {
            // If the next character is non-newline whitespace, skip it.
            curr_offset += 1;
        } else {
            let (token, err) = lex_item(
                state.input,
                &mut curr_offset,
                state.span_offset,
                additional_whitespace,
                special_tokens,
                in_signature,
            );
            if state.error.is_none() {
                state.error = err;
            }
            is_complete = true;
            state.output.push(token);
        }
    }
}

/// True if this the start of a redirection. Does not match `>>` or `>|` forms.
fn is_redirection(token: &[u8]) -> bool {
    matches!(
        token,
        b"o>" | b"out>" | b"e>" | b"err>" | b"o+e>" | b"e+o>" | b"out+err>" | b"err+out>"
    )
}
