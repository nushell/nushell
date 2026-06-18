use nu_protocol::{Span, Spanned, ast::*, engine::StateWorkingSet};

pub(crate) const PERCENT_FORCED_BUILTIN_PARSER_INFO: &str = "percent_forced_builtin";

/// three dots b"..."
pub(crate) const SPREAD_OPERATOR: &[u8; 3] = b"...";

/// three dots "..."
pub(crate) const SPREAD_OPERATOR_STR: &str = "...";

#[inline]
fn extract_spread_value(
    delim: u8,
    Spanned { item, mut span }: Spanned<&[u8]>,
) -> Option<Spanned<&[u8]>> {
    let item = item.strip_prefix(SPREAD_OPERATOR)?;
    span.start += SPREAD_OPERATOR.len();
    match item {
        [b'$' | b'(', ..] => Some(Spanned { item, span }),
        [head, ..] if *head == delim => Some(Spanned { item, span }),
        _ => None,
    }
}

pub(crate) fn extract_spread_list(spanned: Spanned<&[u8]>) -> Option<Spanned<&[u8]>> {
    extract_spread_value(b'[', spanned)
}

pub(crate) fn extract_spread_record(spanned: Spanned<&[u8]>) -> Option<Spanned<&[u8]>> {
    extract_spread_value(b'{', spanned)
}

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
