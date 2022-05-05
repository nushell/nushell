pub fn escape_quote_string(input: &str) -> String {
    let mut output = String::with_capacity(input.len() + 2);
    output.push('"');
    let mut output = String::with_capacity(input.len());
    let is_flag = input.starts_with('-');
    let mut flag_tripped = false;
    for c in input.chars() {
        if c == '"' || c == '\\' {
            output.push('\\');
        }
        output.push(c);
        if (c == ' ' || c == '=') && is_flag && !flag_tripped {
            output.push('"');
            flag_tripped = true;
        }
    }
    if is_flag {
        output.push('"');
    }
    output
}
