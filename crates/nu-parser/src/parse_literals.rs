use crate::{lex::lex, parser::garbage, TokenContents};
use itertools::Itertools;
use log::trace;
use nu_protocol::{ast::*, engine::StateWorkingSet, ParseError, Span, Spanned, Type};
use std::{borrow::Cow, num::ParseIntError, str};

/// Strip underscores for numeric literals
fn strip_underscores(token: &[u8]) -> Cow<[u8]> {
    if let Some(first_underscore) = token.iter().position(|b| *b == b'_') {
        // We assume that we parse numbers (with reasonable len) and only shortly hold the
        // allocation, thus allocate the same size for the owned case.
        let mut owned = Vec::with_capacity(token.len());
        owned.extend_from_slice(&token[..first_underscore]);
        owned.extend(token[first_underscore + 1..].iter().filter(|b| **b != b'_'));

        Cow::Owned(owned)
    } else {
        Cow::Borrowed(token)
    }
}

/// Failed to reinterpet bytes as an ASCII string slice
struct AsciiError;

/// Alternative to `str::from_utf8` if you only care about ASCII characters
///
/// This also allows to shortcircuit following checks if you only expect ASCII characters.
fn parse_ascii_as_str(maybe_ascii: &[u8]) -> Result<&str, AsciiError> {
    if maybe_ascii.is_ascii() {
        // Safe as all ASCII characters are valid UTF-8
        Ok(unsafe { std::str::from_utf8_unchecked(maybe_ascii) })
    } else {
        Err(AsciiError)
    }
}

pub fn parse_int(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let token = working_set.get_span_contents(span);

    fn parse_int_with_base(token: &[u8], span: Span, radix: u32) -> Result<Expression, ParseError> {
        let token = strip_underscores(token);
        if let Ok(token) = parse_ascii_as_str(&token) {
            if let Ok(num) = i64::from_str_radix(token, radix) {
                return Ok(Expression {
                    expr: Expr::Int(num),
                    span,
                    ty: Type::Int,
                    custom_completion: None,
                });
            }
        }
        Err(ParseError::InvalidLiteral(
            format!("invalid digits for radix {}", radix),
            "int".into(),
            span,
        ))
    }

    if token.is_empty() {
        working_set.error(ParseError::Expected("int", span));
        return garbage(span);
    }

    let res = if let Some(num) = token.strip_prefix(b"0b") {
        parse_int_with_base(num, span, 2)
    } else if let Some(num) = token.strip_prefix(b"0o") {
        parse_int_with_base(num, span, 8)
    } else if let Some(num) = token.strip_prefix(b"0x") {
        parse_int_with_base(num, span, 16)
    } else {
        let token = strip_underscores(token);
        parse_ascii_as_str(&token)
            .map_err(|_| ParseError::Expected("int", span))
            .and_then(|s| {
                s.parse::<i64>()
                    .map(|num| Expression {
                        expr: Expr::Int(num),
                        span,
                        ty: Type::Int,
                        custom_completion: None,
                    })
                    .map_err(|_| ParseError::Expected("int", span))
            })
    };

    match res {
        Ok(expr) => expr,
        Err(err) => {
            working_set.error(err);
            garbage(span)
        }
    }
}

pub fn parse_float(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let token = working_set.get_span_contents(span);
    // TODO: we should probably disallow underscores in the special IEEE754 strings
    // e.g. `N_a_N` should not parse as a float
    let token = strip_underscores(token);

    if let Ok(token) = parse_ascii_as_str(&token) {
        if let Ok(x) = token.parse::<f64>() {
            return Expression {
                expr: Expr::Float(x),
                span,
                ty: Type::Float,
                custom_completion: None,
            };
        }
    }
    working_set.error(ParseError::Expected("float", span));
    garbage(span)
}

pub fn parse_number(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let starting_error_count = working_set.parse_errors.len();

    let result = parse_int(working_set, span);
    if starting_error_count == working_set.parse_errors.len() {
        return result;
    } else if !matches!(
        working_set.parse_errors.last(),
        Some(ParseError::Expected(_, _))
    ) {
    } else {
        working_set.parse_errors.truncate(starting_error_count);
    }

    let result = parse_float(working_set, span);

    if starting_error_count == working_set.parse_errors.len() {
        return result;
    }
    working_set.parse_errors.truncate(starting_error_count);

    working_set.error(ParseError::Expected("number", span));
    garbage(span)
}

pub fn parse_binary(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: binary");
    let contents = working_set.get_span_contents(span);
    if contents.starts_with(b"0x[") {
        parse_binary_with_base(working_set, span, 16, 2, b"0x[", b"]")
    } else if contents.starts_with(b"0o[") {
        parse_binary_with_base(working_set, span, 8, 3, b"0o[", b"]")
    } else if contents.starts_with(b"0b[") {
        parse_binary_with_base(working_set, span, 2, 8, b"0b[", b"]")
    } else {
        working_set.error(ParseError::Expected("binary", span));
        garbage(span)
    }
}

fn parse_binary_with_base(
    working_set: &mut StateWorkingSet,
    span: Span,
    base: u32,
    min_digits_per_byte: usize,
    prefix: &[u8],
    suffix: &[u8],
) -> Expression {
    let token = working_set.get_span_contents(span);

    if let Some(token) = token.strip_prefix(prefix) {
        if let Some(token) = token.strip_suffix(suffix) {
            let (lexed, err) = lex(
                token,
                span.start + prefix.len(),
                &[b',', b'\r', b'\n'],
                &[],
                true,
            );
            if let Some(err) = err {
                working_set.error(err);
            }

            let mut binary_value = vec![];
            for token in lexed {
                match token.contents {
                    TokenContents::Item => {
                        let contents = working_set.get_span_contents(token.span);

                        binary_value.extend_from_slice(contents);
                    }
                    TokenContents::Pipe
                    | TokenContents::PipePipe
                    | TokenContents::ErrGreaterPipe
                    | TokenContents::OutGreaterThan
                    | TokenContents::OutErrGreaterPipe
                    | TokenContents::OutGreaterGreaterThan
                    | TokenContents::ErrGreaterThan
                    | TokenContents::ErrGreaterGreaterThan
                    | TokenContents::OutErrGreaterThan
                    | TokenContents::OutErrGreaterGreaterThan => {
                        working_set.error(ParseError::Expected("binary", span));
                        return garbage(span);
                    }
                    TokenContents::Comment | TokenContents::Semicolon | TokenContents::Eol => {}
                }
            }

            let required_padding = (min_digits_per_byte - binary_value.len() % min_digits_per_byte)
                % min_digits_per_byte;

            if required_padding != 0 {
                binary_value = {
                    let mut tail = binary_value;
                    let mut binary_value: Vec<u8> = vec![b'0'; required_padding];
                    binary_value.append(&mut tail);
                    binary_value
                };
            }

            let str = String::from_utf8_lossy(&binary_value).to_string();

            match decode_with_base(&str, base, min_digits_per_byte) {
                Ok(v) => {
                    return Expression {
                        expr: Expr::Binary(v),
                        span,
                        ty: Type::Binary,
                        custom_completion: None,
                    }
                }
                Err(x) => {
                    working_set.error(ParseError::IncorrectValue(
                        "not a binary value".into(),
                        span,
                        x.to_string(),
                    ));
                    return garbage(span);
                }
            }
        }
    }

    working_set.error(ParseError::Expected("binary", span));
    garbage(span)
}

fn decode_with_base(s: &str, base: u32, digits_per_byte: usize) -> Result<Vec<u8>, ParseIntError> {
    s.chars()
        .chunks(digits_per_byte)
        .into_iter()
        .map(|chunk| {
            let str: String = chunk.collect();
            u8::from_str_radix(&str, base)
        })
        .collect()
}

/// Parse a datetime type, eg '2022-02-02'
pub fn parse_datetime(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: datetime");

    let bytes = working_set.get_span_contents(span);

    // Sniff if we start with a year and are at least a date long
    if bytes.len() < 10
        || !bytes[0].is_ascii_digit()
        || !bytes[1].is_ascii_digit()
        || !bytes[2].is_ascii_digit()
        || !bytes[3].is_ascii_digit()
        || bytes[4] != b'-'
    {
        working_set.error(ParseError::Expected("datetime", span));
        return garbage(span);
    }

    // Only ASCII chars are used in a valid RFC3339 date
    let Ok(token) = parse_ascii_as_str(bytes) else {
        working_set.error(ParseError::Expected("datetime", span));
        return garbage(span);
    };

    // Just the date
    // Guaranteed to be 10 bytes long
    // 2024-04-11
    if token.len() == 10 {
        let just_date = token.to_owned() + "T00:00:00+00:00";
        if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(&just_date) {
            return Expression {
                expr: Expr::DateTime(datetime),
                span,
                ty: Type::Date,
                custom_completion: None,
            };
        }
    } else if token.len() > 19 {
        // Happy path for the fully qualified datetime
        // Shortest possible version of RFC3339 has date + time with seconds and local time
        // 2024-04-11T00:00:00Z
        if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(token) {
            return Expression {
                expr: Expr::DateTime(datetime),
                span,
                ty: Type::Date,
                custom_completion: None,
            };
        }
    }

    // Date and time, assume UTC
    // Either 19 bytes
    // 2024-04-11T00:00:00
    // or with fractional seconds
    // where at least 21 bytes are required and the bytes[19] is a `.`
    // 2024-04-11T00:00:00.0
    if token.len() == 19 || (token.len() > 20 && token.as_bytes()[19] == b'.') {
        let datetime = token.to_owned() + "+00:00";
        if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(&datetime) {
            return Expression {
                expr: Expr::DateTime(datetime),
                span,
                ty: Type::Date,
                custom_completion: None,
            };
        }
    }

    working_set.error(ParseError::Expected("datetime", span));

    garbage(span)
}

/// Parse a duration type, eg '10day'
pub fn parse_duration(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: duration");

    let bytes = working_set.get_span_contents(span);

    match parse_unit_value(bytes, span, DURATION_UNIT_GROUPS, Type::Duration, false) {
        Some(Ok(expr)) => expr,
        Some(Err(mk_err_for)) => {
            working_set.error(mk_err_for("duration"));
            garbage(span)
        }
        None => {
            working_set.error(ParseError::Expected("duration with valid units", span));
            garbage(span)
        }
    }
}

/// Parse a unit type, eg '10kb'
pub fn parse_filesize(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: filesize");

    let bytes = working_set.get_span_contents(span);

    // the hex digit `b` might be mistaken for the unit `b`, so check that first
    if bytes.starts_with(b"0x") {
        working_set.error(ParseError::Expected("filesize with valid units", span));
        return garbage(span);
    }

    match parse_unit_value(bytes, span, FILESIZE_UNIT_GROUPS, Type::Filesize, true) {
        Some(Ok(expr)) => expr,
        Some(Err(mk_err_for)) => {
            working_set.error(mk_err_for("filesize"));
            garbage(span)
        }
        None => {
            working_set.error(ParseError::Expected("filesize with valid units", span));
            garbage(span)
        }
    }
}

type ParseUnitResult<'res> = Result<Expression, Box<dyn Fn(&'res str) -> ParseError>>;
type UnitGroup<'unit> = (Unit, &'unit str, Option<(Unit, i64)>);

pub fn parse_unit_value<'res>(
    bytes: &[u8],
    span: Span,
    unit_groups: &[UnitGroup],
    ty: Type,
    ignore_unit_case: bool,
) -> Option<ParseUnitResult<'res>> {
    if bytes.len() < 2
        || !(bytes[0].is_ascii_digit() || (bytes[0] == b'-' && bytes[1].is_ascii_digit()))
    {
        return None;
    }

    // TODO: refactor into trait or proper helper as this certainly not the only use
    fn ends_with_case_insensitive(haystack: &[u8], needle: &[u8]) -> bool {
        // Adapted from `std::slice::ends_with`
        let (m, n) = (haystack.len(), needle.len());
        m >= n && needle.eq_ignore_ascii_case(&haystack[m - n..])
    }

    let unit_filter = |x: &&UnitGroup| {
        if ignore_unit_case {
            ends_with_case_insensitive(bytes, x.1.as_bytes())
        } else {
            bytes.ends_with(x.1.as_bytes())
        }
    };

    if let Some((unit, name, convert)) = unit_groups.iter().find(unit_filter) {
        let lhs_len = bytes.len() - name.len();
        let lhs = strip_underscores(&bytes[..lhs_len]);
        let lhs_span = Span::new(span.start, span.start + lhs_len);
        let unit_span = Span::new(span.start + lhs_len, span.end);
        let Ok(lhs) = parse_ascii_as_str(&lhs) else {
            // We don't expect non-ASCII chars in a number
            return None;
        };
        if lhs.ends_with('$') {
            // If `parse_unit_value` has higher precedence over `parse_range`,
            // a variable with the name of a unit could otherwise not be used as the end of a range.
            return None;
        }

        let (decimal_part, number_part) = modf(match lhs.parse::<f64>() {
            Ok(it) => it,
            Err(_) => {
                let mk_err = move |name| {
                    ParseError::LabeledError(
                        format!("{name} value must be a number"),
                        "not a number".into(),
                        lhs_span,
                    )
                };
                return Some(Err(Box::new(mk_err)));
            }
        });

        let (num, unit) = match convert {
            Some(convert_to) => (
                ((number_part * convert_to.1 as f64) + (decimal_part * convert_to.1 as f64)) as i64,
                convert_to.0,
            ),
            None => (number_part as i64, *unit),
        };

        trace!("-- found {} {:?}", num, unit);
        let expr = Expression {
            expr: Expr::ValueWithUnit(
                Box::new(Expression {
                    expr: Expr::Int(num),
                    span: lhs_span,
                    ty: Type::Number,
                    custom_completion: None,
                }),
                Spanned {
                    item: unit,
                    span: unit_span,
                },
            ),
            span,
            ty,
            custom_completion: None,
        };

        Some(Ok(expr))
    } else {
        None
    }
}

pub const FILESIZE_UNIT_GROUPS: &[UnitGroup] = &[
    (Unit::Kilobyte, "KB", Some((Unit::Byte, 1000))),
    (Unit::Megabyte, "MB", Some((Unit::Kilobyte, 1000))),
    (Unit::Gigabyte, "GB", Some((Unit::Megabyte, 1000))),
    (Unit::Terabyte, "TB", Some((Unit::Gigabyte, 1000))),
    (Unit::Petabyte, "PB", Some((Unit::Terabyte, 1000))),
    (Unit::Exabyte, "EB", Some((Unit::Petabyte, 1000))),
    (Unit::Kibibyte, "KIB", Some((Unit::Byte, 1024))),
    (Unit::Mebibyte, "MIB", Some((Unit::Kibibyte, 1024))),
    (Unit::Gibibyte, "GIB", Some((Unit::Mebibyte, 1024))),
    (Unit::Tebibyte, "TIB", Some((Unit::Gibibyte, 1024))),
    (Unit::Pebibyte, "PIB", Some((Unit::Tebibyte, 1024))),
    (Unit::Exbibyte, "EIB", Some((Unit::Pebibyte, 1024))),
    (Unit::Byte, "B", None),
];

pub const DURATION_UNIT_GROUPS: &[UnitGroup] = &[
    (Unit::Nanosecond, "ns", None),
    // todo start adding aliases for duration units here
    (Unit::Microsecond, "us", Some((Unit::Nanosecond, 1000))),
    (
        // µ Micro Sign
        Unit::Microsecond,
        "\u{00B5}s",
        Some((Unit::Nanosecond, 1000)),
    ),
    (
        // μ Greek small letter Mu
        Unit::Microsecond,
        "\u{03BC}s",
        Some((Unit::Nanosecond, 1000)),
    ),
    (Unit::Millisecond, "ms", Some((Unit::Microsecond, 1000))),
    (Unit::Second, "sec", Some((Unit::Millisecond, 1000))),
    (Unit::Minute, "min", Some((Unit::Second, 60))),
    (Unit::Hour, "hr", Some((Unit::Minute, 60))),
    (Unit::Day, "day", Some((Unit::Minute, 1440))),
    (Unit::Week, "wk", Some((Unit::Day, 7))),
];

// Borrowed from libm at https://github.com/rust-lang/libm/blob/master/src/math/modf.rs
fn modf(x: f64) -> (f64, f64) {
    let rv2: f64;
    let mut u = x.to_bits();
    let e = ((u >> 52 & 0x7ff) as i32) - 0x3ff;

    /* no fractional part */
    if e >= 52 {
        rv2 = x;
        if e == 0x400 && (u << 12) != 0 {
            /* nan */
            return (x, rv2);
        }
        u &= 1 << 63;
        return (f64::from_bits(u), rv2);
    }

    /* no integral part*/
    if e < 0 {
        u &= 1 << 63;
        rv2 = f64::from_bits(u);
        return (x, rv2);
    }

    let mask = ((!0) >> 12) >> e;
    if (u & mask) == 0 {
        rv2 = x;
        u &= 1 << 63;
        return (f64::from_bits(u), rv2);
    }
    u &= !mask;
    rv2 = f64::from_bits(u);
    (x - rv2, rv2)
}

mod test {
    #[test]
    fn test_strip_underscore() {
        use super::*;
        use std::borrow::Cow;

        let basic_text = b"This is regular text.";
        let numeric = b"12345.67";
        let numeric_with_underscores = b"123_45.6_7";
        let just_underscores = b"__";

        assert_eq!(*strip_underscores(basic_text), *basic_text);
        assert!(matches!(strip_underscores(basic_text), Cow::Borrowed(_)));

        assert_eq!(*strip_underscores(numeric), *numeric);
        assert!(matches!(strip_underscores(numeric), Cow::Borrowed(_)));

        assert_eq!(*strip_underscores(numeric_with_underscores), *numeric);

        assert_eq!(*strip_underscores(just_underscores), *b"");
    }
}
