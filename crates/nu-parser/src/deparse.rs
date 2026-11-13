use crate::lex::is_assignment_operator;
use nu_utils::escape_quote_string;

/// Whether `input` needs double quotes to parse back as one string.
fn string_should_be_quoted(input: &str) -> bool {
    input.is_empty()
        || input.starts_with('$')
        || input.chars().any(|c| {
            c.is_whitespace()
                || c == '('
                || c == '['
                || c == '{'
                || c == '}'
                || c == '\''
                || c == '`'
                || c == '"'
                || c == '\\'
                || c == ';'
                || c == '|'
                // `#` starts a comment when it begins a token (or after whitespace).
                // Without quoting, `nu script.nu #000000` becomes `main #000000` and
                // the argument is stripped.
                || c == '#'
        })
}

/// Whether `input`, left bare, parses as code even though [`string_should_be_quoted`] lets it
/// through.
fn string_parses_as_code(input: &str) -> bool {
    // As a whole word, `=`, `+=`, ... parse as an assignment and `&&` is a parse error.
    is_assignment_operator(input.as_bytes())
        || input == "&&"
        // `$` reads a variable (the spread `...$x`, the range `0..$x`), an unmatched `)` is a
        // parse error, and every redirection (`o>`, `e>>`, `out+err>`, `2>`, ...) has a `>`.
        || input.contains(['$', ')', '>'])
}

/// Escapes one command-line argument of `nu script.nu <args>` so that the call `main <args>`
/// parses it back as data, never as code.
///
/// - An argument with no special characters is passed as is (`foo` -> `foo`, `--foo` ->
///   `--foo`), so `3` still parses as an int and `--foo` as a flag.
/// - An argument that is empty, starts with `$`, or contains white space or one of
///   `` ( [ { } ' ` " \ ; | # `` is double-quoted, with `"` and `\` escaped
///   (`two \words` -> `"two \\words"`).
/// - Any other argument that would parse as code is quoted with backticks (`o>` -> `` `o>` ``):
///   one containing `$`, `)` or `>`, an assignment operator (`=`, `+=`, ...), or `&&`. A
///   backtick string reads like a bare word, so a `glob` parameter still globs it and a `path`
///   parameter still expands `~` in it. It can't contain a backtick, since those are
///   double-quoted.
/// - `--name=value` quotes only the value (`--foo=bar'` -> `--foo="bar'"`), so it still parses as
///   the flag `--name`. If the name itself needs quoting, it isn't a flag name, and the whole
///   argument is quoted (`--x;print hi=1` -> `"--x;print hi=1"`, `--x>y=1` -> `` `--x>y=1` ``).
pub fn escape_for_script_arg(input: &str) -> String {
    let (flag, value) = match input
        .strip_prefix("--")
        .and_then(|rest| rest.split_once('='))
    {
        Some((name, value)) if !string_should_be_quoted(name) && !string_parses_as_code(name) => {
            input.split_at(input.len() - value.len())
        }
        _ => ("", input),
    };

    if string_should_be_quoted(value) {
        format!("{flag}{}", escape_quote_string(value))
    } else if string_parses_as_code(value) {
        format!("{flag}`{value}`")
    } else {
        input.into()
    }
}

#[cfg(test)]
mod test {
    use super::escape_for_script_arg;

    #[test]
    fn test_not_extra_quote() {
        // check for input arg like this:
        // nu b.nu word 8
        assert_eq!(escape_for_script_arg("word"), "word".to_string());
        assert_eq!(escape_for_script_arg("8"), "8".to_string());
    }

    #[test]
    fn test_quote_special() {
        let cases = vec![
            ("two words", r#""two words""#),
            ("$nake", r#""$nake""#),
            ("`123", r#""`123""#),
            ("this|cat", r#""this|cat""#),
            ("this;cat", r#""this;cat""#),
            // `#` would start a comment when re-parsed as `main <arg>`
            ("#000000", r##""#000000""##),
            ("#", r##""#""##),
            ("foo#bar", r##""foo#bar""##),
        ];

        for (input, expected) in cases {
            assert_eq!(escape_for_script_arg(input).as_str(), expected);
        }
    }

    // regression test for https://github.com/nushell/nushell/issues/19484
    #[test]
    fn test_quote_code_shapes() {
        let cases = [
            // A flag name with special characters is quoted whole instead of running as code.
            ("--x;print hi", r#""--x;print hi""#),
            ("--x;print hi=1", r#""--x;print hi=1""#),
            ("--help|print hi", r#""--help|print hi""#),
            ("--x\nprint hi", "\"--x\nprint hi\""),
            ("--x>y", "`--x>y`"),
            ("--x$y=1", "`--x$y=1`"),
            // redirections
            ("o>", "`o>`"),
            ("e>>", "`e>>`"),
            ("out+err>", "`out+err>`"),
            ("2>", "`2>`"),
            // variable reads in a spread or a range
            ("...$env.PATH", "`...$env.PATH`"),
            ("0..$nu.pid", "`0..$nu.pid`"),
            ("a$b", "`a$b`"),
            // parse errors when left bare
            ("=", "`=`"),
            ("+=", "`+=`"),
            ("&&", "`&&`"),
            ("a)", "`a)`"),
            // The value after a plain flag name is quoted like any other argument.
            ("--x=...$env.PATH", "--x=`...$env.PATH`"),
            ("--x=o>", "--x=`o>`"),
            // A backtick can't go inside backticks.
            ("a`>", "\"a`>\""),
        ];

        for (input, expected) in cases {
            assert_eq!(escape_for_script_arg(input), expected);
        }
    }

    #[test]
    fn test_plain_words_stay_bare() {
        // Typed literals, paths (`...` is `../..` for a `path` parameter), flags, and the `--`
        // end-of-flags marker must reach the parser unquoted.
        let cases = [
            "3", "-1", "1sec", "5kb", "1..3", "true", "a=b", "~/foo", "...", ".../x", "*.rs", "--",
            "-x", "--x", "--x=1", "--x=a=b",
        ];

        for input in cases {
            assert_eq!(escape_for_script_arg(input), input);
        }
    }

    #[test]
    fn test_flag_value_with_hash() {
        assert_eq!(
            escape_for_script_arg("--color=#000000"),
            r##"--color="#000000""##.to_string()
        );
    }

    #[test]
    fn test_quote_newline() {
        assert_eq!(escape_for_script_arg("c\nd"), format!("\"c\nd\""));
    }

    #[test]
    fn test_arg_with_flag() {
        // check for input arg like this:
        // nu b.nu --linux --version=v5.2
        assert_eq!(escape_for_script_arg("--linux"), "--linux".to_string());
        assert_eq!(
            escape_for_script_arg("--version=v5.2"),
            "--version=v5.2".to_string()
        );

        // check for input arg like this:
        // nu b.nu linux --version v5.2
        assert_eq!(escape_for_script_arg("--version"), "--version".to_string());
        assert_eq!(escape_for_script_arg("v5.2"), "v5.2".to_string());
    }

    #[test]
    fn test_flag_arg_with_values_contains_special() {
        // check for input arg like this:
        // nu b.nu test_ver --version='xx yy' --separator="`"
        assert_eq!(
            escape_for_script_arg("--version='xx yy'"),
            r#"--version="'xx yy'""#.to_string()
        );
        assert_eq!(
            escape_for_script_arg("--separator=`"),
            r#"--separator="`""#.to_string()
        );
    }

    #[test]
    fn test_escape() {
        // check for input arg like this:
        // nu b.nu \ --arg='"'
        assert_eq!(escape_for_script_arg(r"\"), r#""\\""#.to_string());
        assert_eq!(
            escape_for_script_arg(r#"--arg=""#),
            r#"--arg="\"""#.to_string()
        );
    }
}
