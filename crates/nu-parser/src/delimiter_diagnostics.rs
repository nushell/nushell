//! Delimiter error presentation for the lexer.
//!
//! **Invariant:** the delimiter stack alone decides whether lexing failed.
//! Everything in this module only chooses labels, secondary spans, and help
//! text for failures that already exist. Heuristics must never invent an error.

use super::{BlockKind, OpenFrame};
use nu_protocol::{ParseError, Span};

pub(super) fn closing_delimiter_str(block: BlockKind) -> &'static str {
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
/// Presentation may attach a *secondary* lookback hint when the stack failure
/// is likely due to a missing opener nearby (e.g. missing `{` after `if`, or
/// missing `[` before list elements). That never invents a failure — only the
/// stack decides that lexing failed.
///
/// Primary label is always the unmatched closer so "remove this `}`" stays
/// anchored even when the lookback hint is wrong (e.g. matched string content).
pub(super) fn unbalanced_closer(
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
        && let Some((kind, hint_span)) =
            find_missing_open_brace_above(input, span_offset, token_start, close_span)
    {
        // Dual remove/add help: we cannot know whether the user forgot `{` or
        // has an extra `}`. Closer is primary; insert site is secondary.
        let (hint_label, help) = match kind {
            MissingOpenBraceKind::ControlFlow => (
                "possible place for `{` after this condition".into(),
                "Remove this `}` if it is extra, or add `{` after the \
                 `if` / `else if` / `while` / `for` / `try` / `match` condition."
                    .into(),
            ),
            MissingOpenBraceKind::Record => (
                "possible place for `{`".into(),
                "Remove this `}` if it is extra, or add `{` before the record fields \
                 (e.g. `{ key: value }`)."
                    .into(),
            ),
        };
        return ParseError::UnexpectedCloser {
            closer: "}",
            closer_span: close_span,
            hint_span: Some(hint_span),
            hint_label,
            help,
        };
    }

    // Unexpected `]` with no open `[` on the stack (often inside a `{` block).
    // If this line looks like list elements ending in `]` without a `[`, point
    // at where `[` should start — e.g. `2 ($in_ten - 2) 0]`.
    // Do not apply when the top open is `(…` (wrong closer for a group).
    if closer == "]"
        && !matches!(
            block_level.last().map(|f| f.kind),
            Some(BlockKind::SquareBracket | BlockKind::Paren)
        )
        && let Some(hint_span) = find_missing_list_open_bracket(input, span_offset, close_span)
    {
        return ParseError::UnexpectedCloser {
            closer: "]",
            closer_span: close_span,
            hint_span: Some(hint_span),
            hint_label: "possible place for `[`".into(),
            help: "Remove this `]` if it is extra, or add `[` before the list elements \
                   (e.g. `[2 ($in_ten - 2) 0]`)."
                .into(),
        };
    }

    // Unexpected `)` with no open `(` on the stack — e.g. `print -n ansi green)`
    // may have forgotten `(` before `ansi`, or the `)` may simply be extra.
    // Point at a likely insert site; keep help dual-path (remove or add).
    // Do not apply when the top open is `[…` (wrong closer for a list, e.g. `[1, 2, 3)`).
    if closer == ")"
        && !matches!(
            block_level.last().map(|f| f.kind),
            Some(BlockKind::Paren | BlockKind::SquareBracket)
        )
        && let Some(hint_span) = find_missing_open_paren(input, span_offset, close_span)
    {
        return ParseError::UnexpectedCloser {
            closer: ")",
            closer_span: close_span,
            hint_span: Some(hint_span),
            hint_label: "possible place for `(`".into(),
            help: "Remove this `)` if it is extra, or add `(` before the grouped expression."
                .into(),
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

/// Lex-like scanner state for deciding whether a byte is in code vs string/comment.
///
/// Used only on the delimiter-error path so lookback heuristics do not treat
/// multiline string contents (or comments) as control-flow / record syntax.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CodeScanState {
    Code,
    LineComment,
    SingleQuote,
    DoubleQuote,
    Backtick,
    /// Raw string body after `r#…#'`; `n_hashes` is the prefix `#` count.
    RawString {
        n_hashes: usize,
    },
}

/// True when `pos` (byte index into `input`) is outside strings and line comments.
///
/// Scans from the start of `input` so multiline strings opened before a lookback
/// window are still recognized. Mirrors the main lexer's quote / raw-string rules.
fn is_in_code_context(input: &[u8], pos: usize) -> bool {
    if pos >= input.len() {
        return true;
    }
    let mut state = CodeScanState::Code;
    let mut i = 0;
    while i < pos {
        let c = input[i];
        match state {
            CodeScanState::Code => {
                if c == b'#' {
                    state = CodeScanState::LineComment;
                    i += 1;
                } else if c == b'\'' {
                    state = CodeScanState::SingleQuote;
                    i += 1;
                } else if c == b'"' {
                    state = CodeScanState::DoubleQuote;
                    i += 1;
                } else if c == b'`' {
                    state = CodeScanState::Backtick;
                    i += 1;
                } else if c == b'r' && input.get(i + 1) == Some(&b'#') {
                    // `r#…#'…'#…#` — count prefix hashes (same idea as lex_raw_string).
                    let mut n_hashes = 0;
                    let mut j = i + 1;
                    while input.get(j) == Some(&b'#') {
                        n_hashes += 1;
                        j += 1;
                    }
                    if input.get(j) == Some(&b'\'') {
                        state = CodeScanState::RawString { n_hashes };
                        i = j + 1;
                    } else {
                        // Not a valid raw-string opener; treat `r` as code.
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }
            CodeScanState::LineComment => {
                if c == b'\n' {
                    state = CodeScanState::Code;
                }
                i += 1;
            }
            CodeScanState::SingleQuote => {
                if c == b'\'' {
                    state = CodeScanState::Code;
                }
                i += 1;
            }
            CodeScanState::DoubleQuote => {
                if c == b'\\' {
                    // Escape: skip next byte if present.
                    i += 2;
                } else if c == b'"' {
                    state = CodeScanState::Code;
                    i += 1;
                } else {
                    i += 1;
                }
            }
            CodeScanState::Backtick => {
                if c == b'`' {
                    state = CodeScanState::Code;
                }
                i += 1;
            }
            CodeScanState::RawString { n_hashes } => {
                // Closing delimiter is `'` followed by `n_hashes` `#`s.
                // Match when we see the final `#` of that sequence.
                if c == b'#' && n_hashes > 0 {
                    let start = i.saturating_sub(n_hashes);
                    if start < i {
                        let opener_quote = input.get(start);
                        let hashes = &input[start + 1..=i];
                        if opener_quote == Some(&b'\'') && hashes.iter().all(|&b| b == b'#') {
                            state = CodeScanState::Code;
                        }
                    }
                }
                i += 1;
            }
        }
    }
    matches!(state, CodeScanState::Code)
}

/// Shared line context for unexpected-closer lookbacks on a single line.
struct CloserLine<'a> {
    line_start: usize,
    /// Bytes on the line before the closer.
    line: &'a [u8],
    content_start: usize,
}

/// Map a closer span to its current-line code context, or `None` if the closer
/// is not in code or the span is out of range.
fn closer_line_context(
    input: &[u8],
    span_offset: usize,
    close_span: Span,
) -> Option<CloserLine<'_>> {
    let close_local = close_span.start.checked_sub(span_offset)?;
    if close_local > input.len() {
        return None;
    }
    // Do not reshape when the closer itself sits in a string/comment.
    if !is_in_code_context(input, close_local) {
        return None;
    }
    let line_start = input[..close_local]
        .iter()
        .rposition(|&b| b == b'\n' || b == b'\r')
        .map(|i| i + 1)
        .unwrap_or(0);
    let line = &input[line_start..close_local];
    let content_start = line
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(line.len());
    Some(CloserLine {
        line_start,
        line,
        content_start,
    })
}

fn span_at_line_offset(span_offset: usize, line_start: usize, rel: usize) -> Span {
    let abs = span_offset + line_start + rel;
    Span::new(abs, abs + 1)
}

/// On a line ending with an unexpected `]`, if there is no `[` before it on
/// that line, offer an insert-site span for a possible opening bracket.
///
/// Returns a span at the start of the list content (after indent). Callers must
/// keep help dual-path (remove closer or add opener).
fn find_missing_list_open_bracket(
    input: &[u8],
    span_offset: usize,
    close_span: Span,
) -> Option<Span> {
    let ctx = closer_line_context(input, span_offset, close_span)?;
    // No `[` on this line before the `]` → missing open.
    if ctx.line.contains(&b'[') {
        return None;
    }
    if ctx.content_start >= ctx.line.len() {
        return None;
    }
    let abs_local = ctx.line_start + ctx.content_start;
    if !is_in_code_context(input, abs_local) {
        return None;
    }
    // Avoid flagging things that clearly aren't lists (e.g. bare identifiers with ]).
    // Require some list-ish content: digits, `(`, `$`, `"`, `'`, or space-separated values.
    let content = &ctx.line[ctx.content_start..];
    let looks_like_list_elems = content.iter().any(|b| {
        b.is_ascii_digit() || matches!(*b, b'(' | b'$' | b'"' | b'\'' | b'`' | b'-' | b'.' | b' ')
    });
    if !looks_like_list_elems {
        return None;
    }
    Some(span_at_line_offset(
        span_offset,
        ctx.line_start,
        ctx.content_start,
    ))
}

/// On a line ending with an unexpected `)`, if there is no `(` before it on
/// that line, offer an insert-site span for a possible opening paren.
///
/// Returns a span where `(` might be inserted — preferably before the
/// expression being closed (e.g. before `ansi` in `print -n ansi green)`).
/// Callers must keep help dual-path (remove closer or add opener); the closer
/// may simply be extra.
///
/// If the line already contains `(`, do not reshape: balanced groups with an
/// extra `)` (e.g. `print (ansi green))`) must stay plain `Unbalanced`.
fn find_missing_open_paren(input: &[u8], span_offset: usize, close_span: Span) -> Option<Span> {
    let ctx = closer_line_context(input, span_offset, close_span)?;

    // Any `(` on this line before the `)` means this is not a simple missing open
    // (balanced group + extra closer, nested mismatch, etc.).
    if ctx.line.contains(&b'(') {
        return None;
    }

    // Prefer the start of the last "argument group" after a command + flags,
    // so `print -n ansi green)` points at `ansi`, not `print`.
    if let Some(rel) = start_of_trailing_expr_group(ctx.line) {
        let abs_local = ctx.line_start + rel;
        if !is_in_code_context(input, abs_local) {
            return None;
        }
        return Some(span_at_line_offset(span_offset, ctx.line_start, rel));
    }

    if ctx.content_start >= ctx.line.len() {
        return None;
    }
    let abs_local = ctx.line_start + ctx.content_start;
    if !is_in_code_context(input, abs_local) {
        return None;
    }
    Some(span_at_line_offset(
        span_offset,
        ctx.line_start,
        ctx.content_start,
    ))
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
    // Closer inside a string/comment is not a code delimiter failure for lookback.
    if !is_in_code_context(input, close_local) {
        return None;
    }
    // ~2kB lookback is enough for typical "if … / body / }" and bare-record mistakes.
    // String/comment state is established from the start of `input` (see
    // `is_in_code_context`), not from this window alone.
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
        // Absolute index of the first non-whitespace on this line (in `input`).
        let content_rel = line
            .iter()
            .position(|b| !b.is_ascii_whitespace())
            .unwrap_or(0);
        let abs_local = search_from + line_start + content_rel;
        // Skip lines that live inside strings or comments (review: `if` / `type:`
        // inside multiline / raw strings must not reshape the diagnostic).
        if content_rel < line.len() && is_in_code_context(input, abs_local) {
            // Strip trailing comment for the "has `{`?" check (code lines only).
            let code = match line.iter().position(|&b| b == b'#') {
                Some(i) if is_in_code_context(input, search_from + line_start + i) => &line[..i],
                _ => line,
            };
            let trimmed = trim_ascii_start(code);
            if !trimmed.is_empty() {
                if line_looks_like_control_flow_without_brace(trimmed) {
                    // Point at the last non-whitespace of the condition (where `{` belongs).
                    if let Some(span) = span_at_line_end(code, span_offset, search_from, line_start)
                    {
                        // Re-check end-of-line is still code (should match line start).
                        let end_local = span.end.saturating_sub(span_offset);
                        if end_local == 0 || is_in_code_context(input, end_local.saturating_sub(1))
                        {
                            return Some((MissingOpenBraceKind::ControlFlow, span));
                        }
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

/// Identifier continue char. `allow_hyphen` distinguishes keyword boundaries
/// (no `-`) from record keys / structure-hint names (hyphen allowed).
fn is_ident_char(b: u8, allow_hyphen: bool) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || (allow_hyphen && b == b'-')
}

fn is_ascii_ident_continue(b: u8) -> bool {
    is_ident_char(b, false)
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
    key.iter().all(|&b| is_ident_char(b, true))
}

pub(super) fn quote_delimiter_str(quote: u8) -> &'static str {
    match quote {
        b'"' => "\"",
        b'\'' => "'",
        b'`' => "`",
        _ => "\"",
    }
}

/// Best-effort structure hint from bytes immediately before an opening delimiter.
/// Returns a short phrase like `record field ls` or `def foo`, or `None` if unsure.
pub(crate) fn delimiter_structure_hint(bytes_before_open: &[u8]) -> Option<String> {
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
        if is_ident_char(b, true) {
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
    !bytes.is_empty() && bytes.iter().all(|b| is_ident_char(*b, true))
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

pub(super) fn unclosed_from_open(
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

#[cfg(test)]
mod tests {
    use super::delimiter_structure_hint;

    #[test]
    fn structure_hint_record_field() {
        let hint = delimiter_structure_hint(b"  ls: ");
        assert_eq!(hint.as_deref(), Some("record field `ls`"));
    }

    #[test]
    fn structure_hint_def() {
        let hint = delimiter_structure_hint(b"def foo ");
        assert_eq!(hint.as_deref(), Some("`def foo`"));
    }

    #[test]
    fn structure_hint_unsure() {
        assert!(delimiter_structure_hint(b"1 + 2 ").is_none());
    }
}
