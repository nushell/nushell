use nu_protocol::{Span, ast::*, engine::StateWorkingSet};

pub(crate) const PERCENT_FORCED_BUILTIN_PARSER_INFO: &str = "percent_forced_builtin";

pub fn garbage(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    Expression::garbage(working_set, span)
}

pub fn garbage_pipeline(working_set: &mut StateWorkingSet, spans: &[Span]) -> Pipeline {
    Pipeline::from_vec(vec![garbage(working_set, Span::concat(spans))])
}

fn is_identifier_byte(b: &u8) -> bool {
    !b".[({+-*^%/=!<>&|".contains(b)
}

fn is_identifier(bytes: &[u8]) -> bool {
    bytes.iter().all(is_identifier_byte)
}

pub fn is_variable(bytes: &[u8]) -> bool {
    match bytes {
        [b'$', var @ ..] | var if !var.is_empty() => is_identifier(var),
        _ => false,
    }
}

#[rustfmt::skip]
pub fn trim_quotes(bytes: &[u8]) -> &[u8] {
    match bytes {
          [b'\'', trimmed @ .., b'\'']
        | [ b'"', trimmed @ ..,  b'"']
        | [ b'`', trimmed @ ..,  b'`'] => trimmed,
        not_trimmed => not_trimmed,
    }
}

#[rustfmt::skip]
pub fn trim_quotes_str(s: &str) -> &str {
    match s.as_bytes() {
          [b'\'', .., b'\'']
        | [ b'"', ..,  b'"']
        | [ b'`', ..,  b'`'] => &s[1..(s.len() - 1)],
        _ => s,
    }
}
