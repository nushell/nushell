//! Delimiter error presentation for the lexer.
//!
//! **Invariant:** the delimiter stack alone decides whether lexing failed.
//! This module only chooses labels and help text for failures that already
//! exist. It does not invent errors and does not run source lookback heuristics.

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

/// Unexpected closer: report from a real stack failure only.
///
/// Uses the stack top as the unmatched open kind when present; otherwise the
/// default opener for this closer (e.g. `{` for an orphan `}`).
pub(super) fn unbalanced_closer(
    closer: &'static str,
    default_open: &'static str,
    block_level: &[OpenFrame],
    close_span: Span,
) -> ParseError {
    let open = block_level
        .last()
        .map(|f| opening_delimiter_str(f.kind))
        .unwrap_or(default_open);
    ParseError::unbalanced(open, closer, close_span)
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

fn is_ident_char(b: u8, allow_hyphen: bool) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || (allow_hyphen && b == b'-')
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

fn trim_ascii_end(bytes: &[u8]) -> &[u8] {
    let mut end = bytes.len();
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    &bytes[..end]
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
