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

pub fn escape_quote_string_with_file(input: &str, file: &str, count: usize) -> String {
    // use when you want to cross-compare to a file to ensure flags are checked properly
    let file = File::open(file);
    match file {
        Ok(f) => {
            let lines = BufReader::new(f).lines();
            let mut list_of_types = Vec::new();
            let mut list_of_optional_types = Vec::new();
            for line in lines {
                let mut flag_start = false;
                let mut word = String::new();
                let line_or = line.unwrap_or_else(|_| String::from(" "));
                if line_or.contains(':') {
                    let mut type_tripped = 0;
                    let mut word = String::new();
                    for n in line_or.chars() {
                        if n == '?' {
                            type_tripped = 3;
                        }
                        if n == ':' {
                            if type_tripped == 3 {
                                type_tripped = 2;
                            } else {
                                type_tripped = 1;
                            }
                        }
                        if n == ' ' || n == ',' || n == ']' {
                            if type_tripped == 1 {
                                list_of_types.push(word.clone());
                            } else {
                                list_of_optional_types.push(word.clone());
                            }
                            type_tripped = 0;
                        }
                        if type_tripped != 0 && n != ' ' {
                            word.push(n);
                        }
                    }
                }
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
                if word.contains(input) {
                    return input.to_string();
                }
            }

            if !input.contains(' ')
                && !input.contains('=')
                && !input.contains('"')
                && !input.contains('\\')
                && !input.contains('-')
            {
                return input.to_string();
            }
            if input.contains('-')
                && ((count < list_of_types.len()
                    && (list_of_types[count] == "int"
                        || list_of_types[count] == "number"
                        || list_of_types[count] == "math"))
                    || (count >= list_of_types.len()
                        && (list_of_optional_types[count - list_of_types.len()] == "int"
                            || list_of_optional_types[count - list_of_types.len()] == "math"
                            || list_of_optional_types[count - list_of_types.len()] == "number")))
            {
                // if the input is a negative number
                let mut number_quoted = String::from('"');
                number_quoted.push_str(input);
                number_quoted.push('"');
                return number_quoted;
            }
            let mut final_word = String::new();
            final_word.push('"');
            for c in input.chars() {
                if c == '"' || c == '\\' {
                    final_word.push('\\');
                }
                final_word.push(c);
            }
            final_word.push('"');
            final_word
        }
        _ => escape_quote_string_when_flags_are_unclear(input),
    }
}
