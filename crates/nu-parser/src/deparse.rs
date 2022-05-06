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

pub fn escape_quote_string(input: &str) -> String {
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
