use nu_errors::ParseError;
use nu_source::Spanned;

pub(crate) fn trim_quotes(input: &str) -> String {
    let mut chars = input.chars();

    match (chars.next(), chars.next_back()) {
        (Some('\''), Some('\'')) => chars.collect(),
        (Some('"'), Some('"')) => chars.collect(),
        (Some('`'), Some('`')) => chars.collect(),
        _ => input.to_string(),
    }
}

pub(crate) fn verify_and_strip(
    contents: &Spanned<String>,
    left: char,
    right: char,
) -> (String, Option<ParseError>) {
    let mut chars = contents.item.chars();

    match (chars.next(), chars.next_back()) {
        (Some(l), Some(r)) if l == left && r == right => {
            let output: String = chars.collect();
            (output, None)
        }
        _ => (
            String::new(),
            Some(ParseError::mismatch(
                format!("value in {} {}", left, right),
                contents.clone(),
            )),
        ),
    }
}
