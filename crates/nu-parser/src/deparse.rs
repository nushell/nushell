use nu_utils::escape_quote_string;

fn string_should_be_quoted(input: &str) -> bool {
    input.starts_with('$')
        || input.chars().any(|c| {
            c == ' '
                || c == '('
                || c == '\''
                || c == '`'
                || c == '"'
                || c == '\\'
                || c == ';'
                || c == '|'
        })
}

// Escape rules:
// input argument is not a flag, does not start with $ and doesn't contain special characters, it is passed as it is (foo -> foo)
// input argument is not a flag and either starts with $ or contains special characters, quotes are added, " and \ are escaped (two \words -> "two \\words")
// input argument is a flag without =, it's passed as it is (--foo -> --foo)
// input argument is a flag with =, the first two points apply to the value (--foo=bar -> --foo=bar; --foo=bar' -> --foo="bar'")
//
// special characters are white space, (, ', `, ",and \
pub fn escape_for_script_arg(input: &str) -> String {
    // handle for flag, maybe we need to escape the value.
    if input.starts_with("--") {
        if let Some((arg_name, arg_val)) = input.split_once('=') {
            // only want to escape arg_val.
            let arg_val = if string_should_be_quoted(arg_val) {
                escape_quote_string(arg_val)
            } else {
                arg_val.into()
            };

            return format!("{arg_name}={arg_val}");
        } else {
            return input.into();
        }
    }
    if string_should_be_quoted(input) {
        escape_quote_string(input)
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
        ];

        for (input, expected) in cases {
            assert_eq!(escape_for_script_arg(input).as_str(), expected);
        }
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
