use std::fs::File;
use std::io::{BufRead, BufReader};

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
        Ok(f) => {
            let lines = BufReader::new(f).lines();
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
                if word == input {
                    return word;
                }
            }
            let mut final_word = String::new();
            final_word.push('"');
            final_word.push_str(input);
            final_word.push('"');
            final_word
        }
        _ => escape_quote_string_when_flags_are_unclear(input),
    }
}
