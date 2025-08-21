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
