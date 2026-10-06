use fancy_regex::Regex;
use std::sync::LazyLock;

// This hits, in order:
// • Any character of []:`{}#'";()|$,.!?=
// • Any digit (\d)
// • Any whitespace (\s)
// • A NUL, which must be escaped rather than written raw
// • Case-insensitive sign-insensitive float "keywords" inf, infinity and nan.
static NEEDS_QUOTING_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"[\[\]:`\{\}#'";\(\)\|\$,\.\d\s\x00!?=]|(?i)^[+\-]?(inf(inity)?|nan)$"#)
        .expect("internal error: NEEDS_QUOTING_REGEX didn't compile")
});

pub fn needs_quoting(string: &str) -> bool {
    if string.is_empty() {
        return true;
    }
    // These are case-sensitive keywords
    match string {
        // `true`/`false`/`null` are active keywords in JSON and NUON
        // `&&` is denied by the nu parser for diagnostics reasons
        // (https://github.com/nushell/nushell/pull/7241)
        "true" | "false" | "null" | "&&" => return true,
        _ => (),
    };
    // All other cases are handled here
    NEEDS_QUOTING_REGEX.is_match(string).unwrap_or(false)
}

pub fn escape_quote_string(string: &str) -> String {
    let mut output = String::with_capacity(string.len() + 2);
    output.push('"');

    for c in string.chars() {
        match c {
            '"' | '\\' => {
                output.push('\\');
                output.push(c);
            }
            // A raw NUL is rejected by the reader, so the writer must escape it rather
            // than emit a byte its own reader refuses.
            '\0' => output.push_str("\\0"),
            _ => output.push(c),
        }
    }

    output.push('"');
    output
}

/// Returns a raw string representation if the string contains quotes or backslashes.
/// Otherwise returns None (caller should use regular quoting or bare string).
///
/// Raw strings avoid escaping by using `r#'...'#` syntax with enough `#` characters
/// to ensure the closing delimiter is unambiguous.
///
/// Note: Nushell requires at least one `#` in raw strings (i.e., `r#'...'#` not `r'...'`).
pub fn as_raw_string(s: &str) -> Option<String> {
    // Only use raw strings if they would avoid escaping
    if !s.contains('"') && !s.contains('\\') {
        return None;
    }

    // A raw string reproduces its content byte for byte, so it cannot carry the
    // `\0` escape: a NUL inside `r#'...'#` stays a raw NUL in the output, which is
    // exactly what the caller is escaping. Refuse the raw form and let the value
    // fall through to `escape_quote_string`, which writes `\0`.
    if s.contains('\0') {
        return None;
    }

    // Find minimum # count needed for delimiter.
    // Nushell requires at least one #, so start at 1.
    // Need to avoid both:
    // - `'#...#` patterns in content that would close early
    // - leading `###...` content, because the opening quote plus the first
    //   `###` would also be parsed as a closing delimiter
    let mut hash_count = 1;
    loop {
        let hashes = "#".repeat(hash_count);
        let closing = format!("'{}", hashes);

        if !s.starts_with(&hashes) && !s.contains(&closing) {
            return Some(format!("r{hashes}'{s}'{hashes}"));
        }

        hash_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::{as_raw_string, escape_quote_string, needs_quoting};

    #[test]
    fn escape_quote_string_escapes_nul_as_backslash_zero() {
        // The reader is specified to reject a raw NUL, so the writer must not emit one.
        assert_eq!(escape_quote_string("a\0b"), r#""a\0b""#);
        assert_eq!(escape_quote_string("\0"), r#""\0""#);
        assert!(
            !escape_quote_string("\0").contains('\0'),
            "the escaped form must not contain a raw NUL"
        );
    }

    #[test]
    fn escape_quote_string_still_escapes_quotes_and_backslashes() {
        // The NUL arm must not have displaced the existing escapes.
        assert_eq!(escape_quote_string(r#"a"b"#), r#""a\"b""#);
        assert_eq!(escape_quote_string(r"a\b"), r#""a\\b""#);
        assert_eq!(escape_quote_string("a\0b\"c"), r#""a\0b\"c""#);
    }

    #[test]
    fn needs_quoting_forces_a_nul_to_be_quoted() {
        // Without this, a NUL-bearing string is written as a bare word with a raw NUL
        // inside it, which no conforming reader will accept.
        assert!(needs_quoting("a\0b"));
        assert!(needs_quoting("\0"));
        // ...and nothing else changed: ordinary strings still take the fast path.
        assert!(!needs_quoting("hello"));
        assert!(!needs_quoting("plain_name"));
    }

    #[test]
    fn raw_string_uses_single_hash_when_safe() {
        assert_eq!(
            as_raw_string(r#"hello \"world\""#),
            Some(r#"r#'hello \"world\"'#"#.to_string())
        );
    }

    #[test]
    fn raw_string_is_refused_when_the_value_has_a_nul() {
        // A raw string cannot represent the `\0` escape, so one would come back out
        // as a raw NUL. Refusing it hands the value to `escape_quote_string` instead.
        for value in ["a\0\"b", "\0\"", "a\\b\0"] {
            assert_eq!(
                as_raw_string(value),
                None,
                "a NUL-bearing value must not take the raw form: {value:?}"
            );
        }

        // And nothing else changed: a NUL-free value that needs escaping still gets it.
        assert_eq!(
            as_raw_string(r#"hello \"world\""#),
            Some(r#"r#'hello \"world\"'#"#.to_string())
        );
    }

    #[test]
    fn raw_string_uses_more_hashes_for_quote_hash_sequence() {
        assert_eq!(
            as_raw_string(r#"contains '# and "quote""#),
            Some(r##"r##'contains '# and "quote"'##"##.to_string())
        );
    }

    #[test]
    fn raw_string_uses_more_hashes_when_content_starts_with_hash() {
        let input = "# example.toml\nname = \"my-app\"\nversion = \"1.0.0\"\n";

        assert_eq!(
            as_raw_string(input),
            Some(
                r##"r##'# example.toml
name = "my-app"
version = "1.0.0"
'##"##
                    .to_string()
            )
        );
    }

    #[test]
    fn raw_string_scales_hash_count_for_longer_sequences() {
        assert_eq!(
            as_raw_string(r#"contains '## and "quote""#),
            Some(r###"r###'contains '## and "quote"'###"###.to_string())
        );
    }
}
