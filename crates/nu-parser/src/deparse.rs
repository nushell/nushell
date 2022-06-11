use std::fs::File;
use std::io::{BufRead, BufReader, Read};

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

fn looks_like_flag(input: &str) -> bool {
    if !input.starts_with('-') {
        false
        // it does not start with  '-'
    } else if !input.starts_with("--") {
        if input.len() > 2
            && input.chars().nth(2).expect("this should never trigger") != '='
            && input.chars().nth(2).expect("this should never trigger") != ' '
        {
            false
            // while it start with '-', it is not of the form '-x=y' or '-x y'
        } else {
            input.len() >= 2
        }
    } else {
        input.len() > 2
        // it is either a flag --x or a '--'
    }
}

fn escape_quote_string_when_flags_are_unclear(input: &str) -> String {
    // internal use only. When reading the file for flags goes wrong, revert back to a manual check
    // for flags.
    let mut output = String::new();
    if !looks_like_flag(input) {
        output.push('"');
        for c in input.chars() {
            if c == '"' || c == '\\' {
                output.push('\\');
            }
            output.push(c);
        }
        output.push('"');
        output
    } else if input.contains(' ') || input.contains('=') {
        // this is a flag that requires delicate handling
        let mut flag_tripped = false;
        for c in input.chars() {
            if c == '"' || c == '\\' {
                output.push('\\');
            }
            output.push(c);
            if (c == ' ' || c == '=') && !flag_tripped {
                flag_tripped = true;
                output.push('"');
            }
        }
        output.push('"');
        output
    } else {
        // this is a normal flag, aka "--x"
        String::from(input)
    }
}

pub fn escape_quote_string_with_file(input: &str, file: &str) -> String {
    // use when you want to cross-compare to a file to ensure flags are checked properly
    let file = File::open(file);
    match file {
        Ok(f) => escape_quote_string_with_reader(f, input),
        _ => escape_quote_string_when_flags_are_unclear(input),
    }
}

fn escape_quote_string_with_reader<R>(input: R, input_arg: &str) -> String
where
    R: Read,
{
    let lines = BufReader::new(input).lines();

    for line in lines {
        let mut flag_start = false;
        let mut word = String::new();
        let line_or = line.unwrap_or_else(|_| String::from(" "));
        if line_or.contains('-') {
            for n in line_or.chars() {
                if n == '-' {
                    flag_start = true;
                }
                if n == ' ' || n == ':' || n == ')' {
                    flag_start = false;
                }
                if flag_start {
                    word.push(n);
                }
            }
        }

        if word.contains(input_arg) {
            return input_arg.to_string();
        } else if let Some((arg_name, arg_val)) = input_arg.split_once('=') {
            if word.contains(arg_name) && arg_val.contains(' ') {
                return format!("{}=`{}`", arg_name, arg_val);
            }
        }
    }
    if input_arg.contains(' ') {
        format!("`{}`", input_arg)
    } else if input_arg.contains('"') || input_arg.contains('\\') {
        escape_quote_string(input_arg)
    } else {
        input_arg.to_string()
    }
}

#[cfg(test)]
mod test {
    use super::escape_quote_string_with_reader;

    #[test]
    fn test_not_extra_quote() {
        let body = r#"
        def main [x: int] {
            $x + 10
        }
        "#;
        let input_file = body.as_bytes();
        // check for input arg like this:
        // nu b.nu 8
        assert_eq!(
            escape_quote_string_with_reader(input_file, "8"),
            "8".to_string()
        );
    }

    #[test]
    fn test_arg_with_flag() {
        let body = r#"
        def main [kernel: string, --version: string] {
            echo $kernel;
            echo $version
        }
        "#;
        let input_file = body.as_bytes();
        // check for input arg like this:
        // nu b.nu linux --version=v5.2
        assert_eq!(
            escape_quote_string_with_reader(input_file, "linux"),
            "linux".to_string()
        );
        assert_eq!(
            escape_quote_string_with_reader(input_file, "--version=v5.2"),
            "--version=v5.2".to_string()
        );

        // check for input arg like this:
        // nu b.nu linux --version v5.2
        assert_eq!(
            escape_quote_string_with_reader(input_file, "--version"),
            "--version".to_string()
        );
        assert_eq!(
            escape_quote_string_with_reader(input_file, "v5.2"),
            "v5.2".to_string()
        );
    }

    #[test]
    fn test_flag_arg_with_values_contains_space() {
        let body = r#"
        def main [kernel: string, --version: string, --arch: string] {
            echo $kernel
            echo $version
            echo $arch
        }
        "#;
        let input_file = body.as_bytes();

        // check for input arg like this:
        // nu b.nu test_ver --version='xx yy' --arch=ghi
        assert_eq!(
            escape_quote_string_with_reader(input_file, "--version='xx yy'"),
            "--version=`'xx yy'`".to_string()
        );
        assert_eq!(
            escape_quote_string_with_reader(input_file, "--arch=ghi"),
            "--arch=ghi".to_string()
        );
    }

    #[test]
    fn test_escape() {
        let body = r#"
        def main [x: any] {
            echo $x
        }"#;
        let input_file = body.as_bytes();

        // check for input arg like this:
        // nu b.nu '"'
        assert_eq!(
            escape_quote_string_with_reader(input_file, r#"""#),
            r#""\"""#.to_string()
        );
    }
}
