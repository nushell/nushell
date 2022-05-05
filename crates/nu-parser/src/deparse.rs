fn parse_input(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for c in input.chars() {
        if c == '"' || c == '\\' {
            output.push('\\');
        }
        output.push(c);
    }
    output
}

pub fn escape_quote_string(input: &str) -> String {
    if input.starts_with('-') {
        parse_input(input)
    } else {
        let mut output = String::with_capacity(input.len() + 2);
        output.push('"');

        output.push_str(&parse_input(input));

        output.push('"');
        output
    }
}
