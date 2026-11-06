//! The construct before an unclosed delimiter, which its error's help names (nu-parser's
//! `delimiter_structure_hint`).

/// The keywords whose construct a hint names.
const KEYWORDS: [&str; 10] = [
    "def", "module", "extern", "if", "match", "try", "for", "while", "loop", "export",
];

/// A short phrase naming the construct that `before`, the text before an opening delimiter,
/// ends with, as nu names it: ``record field `ls` `` for `ls: `, `` `def foo` `` for `def foo `
/// (and the same after `export`), a keyword alone for another of [`KEYWORDS`]; `None` when
/// unsure. Only the last line of the last 80 bytes is read.
pub(super) fn delimiter_structure_hint(before: &[u8]) -> Option<String> {
    const WINDOW: usize = 80;
    let window = &before[before.len().saturating_sub(WINDOW)..];
    let line = match window
        .iter()
        .rposition(|&byte| byte == b'\n' || byte == b'\r')
    {
        Some(newline) => &window[newline + 1..],
        None => window,
    };
    let line = line.trim_ascii_end();
    if line.is_empty() {
        return None;
    }
    // A record field: `name:`, the opener after it.
    if let Some(colon) = line.iter().rposition(|&byte| byte == b':')
        && let Some(name) = trailing_ident(line[..colon].trim_ascii_end())
    {
        return Some(format!("record field `{name}`"));
    }
    let words: Vec<&str> = line
        .split(u8::is_ascii_whitespace)
        .filter(|word| !word.is_empty())
        .filter_map(|word| std::str::from_utf8(word).ok())
        .collect();
    let (&first, rest) = words.split_first()?;
    let named = |keyword: &str| matches!(keyword, "def" | "module" | "extern");
    let name = |index: usize| rest.get(index).filter(|name| is_simple_ident(name));
    if first == "export"
        && let Some(&keyword) = rest.first()
        && named(keyword)
    {
        return Some(match name(1) {
            Some(name) => format!("`export {keyword} {name}`"),
            None => format!("`export {keyword}`"),
        });
    }
    if !KEYWORDS.contains(&first) {
        return None;
    }
    Some(match name(0).filter(|_| named(first)) {
        Some(name) => format!("`{first} {name}`"),
        None => format!("`{first}`"),
    })
}

/// The identifier `bytes` end with, blanks after it aside; `None` without one.
fn trailing_ident(bytes: &[u8]) -> Option<&str> {
    let bytes = bytes.trim_ascii_end();
    let start = bytes
        .iter()
        .rposition(|&byte| !is_ident_byte(byte))
        .map_or(0, |last| last + 1);
    std::str::from_utf8(&bytes[start..])
        .ok()
        .filter(|ident| !ident.is_empty())
}

/// Whether `text` is an identifier: letters, digits, `_` and `-`.
fn is_simple_ident(text: &&str) -> bool {
    !text.is_empty() && text.bytes().all(is_ident_byte)
}

/// Whether `byte` may be part of an identifier.
fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
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
    fn structure_hint_export_def() {
        let hint = delimiter_structure_hint(b"x\nexport def foo ");
        assert_eq!(hint.as_deref(), Some("`export def foo`"));
        let hint = delimiter_structure_hint(b"export def 'foo' ");
        assert_eq!(hint.as_deref(), Some("`export def`"));
        let hint = delimiter_structure_hint(b"export use ");
        assert_eq!(hint.as_deref(), Some("`export`"));
    }

    #[test]
    fn structure_hint_unsure() {
        assert!(delimiter_structure_hint(b"1 + 2 ").is_none());
        assert!(delimiter_structure_hint(b"x\n  ").is_none());
    }
}
