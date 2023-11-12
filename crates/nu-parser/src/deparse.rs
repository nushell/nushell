pub fn escape_quote_string(input: &str) -> String {
    let mut output = String::with_capacity(input.len() + 2);
    output.push('"');

    for c in input.chars() {
        if c == '"' || c == '\\' {
            output.push('\\');
        }
        output.push(c);
    }

    output.push('"');
    output
}

// Escape rules:
// input argument is not a flag and contains only alphanumeric characters, it is passed as it is (foo -> foo)
// input argument is not a flag and contains non-alphanumeric characters, quotes are added, " and \ are escaped (two \words -> "two \\words")
// input argument is a flag without =, it's passed as it is (--foo -> --foo)
// input argument is a flag with =, the first two points apply to the value (--foo=bar -> --foo=bar; --foo=bar_ -> --foo="bar_")
//
// quotations are needed in case of some characters (like ', `, (, $) to avoid reinterpretation during parsing which leads to errors
pub fn escape_for_script_arg(input: &str) -> String {
    // handle for flag, maybe we need to escape the value.
    if input.starts_with("--") {
        if let Some((arg_name, arg_val)) = input.split_once('=') {
            // only want to escape arg_val.
            let arg_val = if arg_val.chars().all(|character| character.is_alphanumeric()) {
                arg_val.into()
            } else {
                escape_quote_string(arg_val)
            };

            return format!("{arg_name}={arg_val}");
        } else {
            return input.into();
        }
    }
    if input.chars().all(|character| character.is_alphanumeric()) {
        input.into()
    } else {
        escape_quote_string(input)
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
    fn test_quote_non_alphanumeric() {
        // check for input arg like this:
        // nu b.nu "two words" ma$$ "it's"
        assert_eq!(
            escape_for_script_arg("two words"),
            r#""two words""#.to_string()
        );
        assert_eq!(escape_for_script_arg("ma$$"), r#""ma$$""#.to_string());
        assert_eq!(escape_for_script_arg("it's"), r#""it's""#.to_string());
    }

    #[test]
    fn test_arg_with_flag() {
        // check for input arg like this:
        // nu b.nu --linux --version=v5.2 --language=en
        assert_eq!(escape_for_script_arg("--linux"), "--linux".to_string());
        assert_eq!(
            escape_for_script_arg("--version=v5.2"),
            r#"--version="v5.2""#.to_string()
        );
        assert_eq!(
            escape_for_script_arg("--language=en"),
            "--language=en".to_string()
        );

        // check for input arg like this:
        // nu b.nu linux --version v5.2
        assert_eq!(escape_for_script_arg("--version"), "--version".to_string());
        assert_eq!(escape_for_script_arg("v5.2"), r#""v5.2""#.to_string());
    }

    #[test]
    fn test_flag_arg_with_values_contains_non_alphanumeric() {
        // check for input arg like this:
        // nu b.nu test_ver --version='xx yy'
        assert_eq!(
            escape_for_script_arg("--version='xx yy'"),
            r#"--version="'xx yy'""#.to_string()
        );
        assert_eq!(
            escape_for_script_arg("--arch=ghi"),
            "--arch=ghi".to_string()
        );
    }

    #[test]
    fn test_escape() {
        // check for input arg like this:
        // nu b.nu '"'
        assert_eq!(escape_for_script_arg(r#"""#), r#""\"""#.to_string());
    }
}
