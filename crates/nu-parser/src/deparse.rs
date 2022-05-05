pub fn escape_quote_string(input: &str) -> String {
    if input.starts_with('-') {
        String::from(input)
    } else {
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
}
