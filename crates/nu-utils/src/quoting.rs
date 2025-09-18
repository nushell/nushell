use fancy_regex::Regex;
use std::sync::LazyLock;

// This hits, in order:
// • Any character of []:`{}#'";()|$,.!?=
// • Any digit (\d)
// • Any whitespace (\s)
// • Case-insensitive sign-insensitive float "keywords" inf, infinity and nan.
static NEEDS_QUOTING_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"[\[\]:`\{\}#'";\(\)\|\$,\.\d\s!?=]|(?i)^[+\-]?(inf(inity)?|nan)$"#)
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
        if c == '"' || c == '\\' {
            output.push('\\');
        }
        output.push(c);
    }

    output.push('"');
    output
}

enum State {
    Unquoted,
    InQuote(char),
}

/// Splits a string into groups separated by whitespace, but keeps text inside quotes together.
/// 
/// Supports `'`, `"`, and `` ` `` as quotes. Backslashes escape quotes (except inside backticks),
/// and escaped characters are treated as part of the group.
/// 
/// Example:
/// ```rust
/// assert_eq!(
///     split_quote_groups("foo 'bar baz' qux"),
///     vec!["foo", "'bar baz'", "qux"]
/// );
/// ```
/// 
/// This function does not remove the quotes. It just groups the input so the caller can
/// decide what to do with them later.
pub fn split_quote_groups(input: &str) -> Vec<&str> {
    let mut out = Vec::with_capacity(4); // gut feeling start

    let mut group_start = 0;
    let mut state = State::Unquoted;
    let mut escaped = false;

    for (i, c) in input.char_indices() {
        match state {
            State::Unquoted => match c {
                c if c.is_whitespace() => {
                    out.push(&input[group_start..i]);
                    group_start = i + 1;
                }
                '\'' | '"' | '`' if !escaped => state = State::InQuote(c),
                '\\' => escaped = !escaped,
                _ => escaped = false,
            },
            State::InQuote(q) => match c {
                c if c == q && !escaped => state = State::Unquoted,
                '\\' if q != '`' => escaped = !escaped,
                _ => escaped = false,
            },
        }
    }

    out.push(&input[group_start..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    use split_quote_groups as sqg;

    #[test]
    fn split_quote_groups_works() {
        assert_eq!(sqg("abc"), vec!["abc"]);
        assert_eq!(sqg("abc def"), vec!["abc", "def"]);
        assert_eq!(
            sqg(r#""with space" without"#),
            vec![r#""with space""#, "without"]
        );
        assert_eq!(sqg(r#""with 'quote'""#), vec![r#""with 'quote'""#]);
        assert_eq!(
            sqg(r#"`escaping\` doesn't\matter`"#),
            vec![r#"`escaping\`"#, r#"doesn't\matter`"#]
        );
        assert_eq!(
            sqg(r#""escaping \" does" matter"#),
            vec![r#""escaping \" does""#, "matter"]
        );
    }

    #[test]
    fn empty_input_is_single_empty_group() {
        assert_eq!(sqg(""), vec![""]);
    }

    #[test]
    fn preserves_empty_groups_from_whitespace_runs() {
        assert_eq!(sqg("  a   b  "), vec!["", "", "a", "", "", "b", "", ""]);
        assert_eq!(sqg("a  b   c"), vec!["a", "", "b", "", "", "c"]);
        assert_eq!(sqg("a \t  b"), vec!["a", "", "", "", "b"]);
    }

    #[test]
    fn leading_and_trailing_whitespace_create_empty_edges() {
        assert_eq!(sqg(" a"), vec!["", "a"]);
        assert_eq!(sqg("a "), vec!["a", ""]);
        assert_eq!(sqg(" a "), vec!["", "a", ""]);
    }

    #[test]
    fn all_whitespace_is_all_empties_plus_final_empty() {
        // three spaces -> 4 empty groups
        assert_eq!(sqg("   "), vec!["", "", "", ""]);
    }

    #[test]
    fn consecutive_whitespace_between_quotes_keeps_internal_empties() {
        assert_eq!(sqg(r#""a"  "b""#), vec![r#""a""#, "", r#""b""#]);
    }

    #[test]
    fn escaped_space_outside_quotes_is_still_a_split() {
        assert_eq!(sqg(r"a\ b"), vec![r"a\", "b"]);
        assert_eq!(sqg(r"one\ two  three"), vec![r"one\", "two", "", "three"]);
    }
}
