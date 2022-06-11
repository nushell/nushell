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
// input argument contains ' '(like abc def), we will convert it to `abc def`.
// input argument contains --version='xx yy', we will convert it to --version=`'xx yy'`
// input argument contains " or \, we will try to escape input.
pub fn escape_for_script_arg(input: &str) -> String {
    // handle for flag, maybe we need to escape the value.
    if input.starts_with("--") {
        if let Some((arg_name, arg_val)) = input.split_once('=') {
            // only want to escape arg_val.
            let arg_val = if arg_val.contains(' ') {
                format!("`{}`", arg_val)
            } else if arg_val.contains('"') || arg_val.contains('\\') {
                escape_quote_string(arg_val)
            } else {
                arg_val.into()
            };
            return format!("{}={}", arg_name, arg_val);
        }
    }

    if input.contains(' ') {
        format!("`{}`", input)
    } else if input.contains('"') || input.contains('\\') {
        escape_quote_string(input)
    } else {
        input.to_string()
    }
}

#[cfg(test)]
mod test {
    use super::escape_for_script_arg;

    #[test]
    fn test_not_extra_quote() {
        // check for input arg like this:
        // nu b.nu 8
        assert_eq!(escape_for_script_arg("8"), "8".to_string());
    }

    #[test]
    fn test_arg_with_flag() {
        // check for input arg like this:
        // nu b.nu linux --version=v5.2
        assert_eq!(escape_for_script_arg("linux"), "linux".to_string());
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
    fn test_flag_arg_with_values_contains_space() {
        // check for input arg like this:
        // nu b.nu test_ver --version='xx yy' --arch=ghi
        assert_eq!(
            escape_for_script_arg("--version='xx yy'"),
            "--version=`'xx yy'`".to_string()
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
