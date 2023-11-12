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
// input argument is a flag with =, we will wrap the value of flag in quotes and escape \ and " (--version=1\3 -> --vesion="1\\3")
// input argument is a flag without =, we will pass it as it is (--version -> --version)
// input argument is not a flag, we will wrap it in quotes and escape \ and " (some other "argument" -> "some other \"argument\"")
pub fn escape_for_script_arg(input: &str) -> String {
    // handle for flag, maybe we need to escape the value.
    if input.starts_with("--") {
        if let Some((arg_name, arg_val)) = input.split_once('=') {
            // only want to escape arg_val.
            let arg_val = escape_quote_string(arg_val);
            return format!("{arg_name}={arg_val}");
        } else {
            return input.to_string();
        }
    }

    escape_quote_string(input)
}

#[cfg(test)]
mod test {
    use super::escape_for_script_arg;

    #[test]
    fn test_single_character() {
        // check for input arg like this:
        // nu b.nu 8
        assert_eq!(escape_for_script_arg("8"), r#""8""#.to_string());
    }

    #[test]
    fn test_empty_string() {
        // check for empty string as an argument
        assert_eq!(escape_for_script_arg(""), r#""""#.to_string());
    }

    #[test]
    fn test_arg_with_flag() {
        // check for input arg like this:
        // nu b.nu linux --version=v5.2
        assert_eq!(escape_for_script_arg("linux"), "\"linux\"".to_string());
        assert_eq!(
            escape_for_script_arg("--version=v5.2"),
            r#"--version="v5.2""#.to_string()
        );

        // check for input arg like this:
        // nu b.nu linux --version v5.2
        assert_eq!(escape_for_script_arg("--version"), "--version".to_string());
        assert_eq!(escape_for_script_arg("v5.2"), r#""v5.2""#.to_string());
    }

    #[test]
    fn test_flag_arg_with_values_contains_space() {
        // check for input arg like this:
        // nu b.nu test_ver --version='xx yy'
        assert_eq!(
            escape_for_script_arg("--version='xx yy'"),
            r#"--version="'xx yy'""#.to_string()
        );
    }

    #[test]
    fn test_escape() {
        // check for input arg like this:
        // nu b.nu '"\'
        assert_eq!(escape_for_script_arg(r#""\"#), r#""\"\\""#.to_string());
    }

    #[test]
    fn test_special_characters() {
        // check for input arg like this:
        // nu b.nu "$(`'"
        assert_eq!(escape_for_script_arg(r#"$(`'"#), r#""$(`'""#.to_string());
    }
}
