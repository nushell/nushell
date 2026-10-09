//! Literals: numbers, units, datetimes, binary, strings and interpolation,
//! variables, cell paths and ranges (nu-parser's `parse_literals.rs`).
//!
//! Most of these take the text of one lexed item and either recognise it
//! completely or fail; [`super::parse_expressions::parse_value`] tries them in
//! Nushell's order.

use std::borrow::Cow;

use winnow::ascii::digit1;
use winnow::combinator::{alt, delimited, opt, preceded, repeat};
use winnow::error::{EmptyError, ErrMode};
use winnow::prelude::*;
use winnow::token::{any, one_of, rest, take_till, take_while};

use crate::ast::{
    BinaryLiteral, CellPath, Duration, DurationUnit, Expr, Expression, Filesize, FilesizeUnit, FullCellPath,
    InterpolationPart, PathMember, PathMemberKind, Quote, Range, RangeInclusion, RangeOperator, StringInterpolation,
    StringLiteral, Var,
};
use crate::error::{Diagnostic, ErrorKind};
use crate::input::{Input, ParseFailure, ParseResult, cut, input, into_diagnostic, pos, rest_span};
use crate::lex::{LexOptions, Token, TokenContents, group_end, interp_subexpr_step, lex};
use crate::span::Span;

use super::WorkingSet;
use super::parse_expressions::{ExpectedShape, parse_list_expression, parse_record, parse_subexpression, parse_value};
use super::parse_helpers::is_identifier;
use super::tokens::{Tokens, item, keyword};

/// Remove `_` digit separators, borrowing when there are none.
fn strip_underscores(text: &str) -> Cow<'_, str> {
    if text.contains('_') { Cow::Owned(text.replace('_', "")) } else { Cow::Borrowed(text) }
}

/// An integer literal (nu's `parse_int`), with `_` separators anywhere:
///
/// ```text
/// int = decimal | "0x" hex-digits | "0o" octal-digits | "0b" binary-digits
/// ```
///
/// Like Nushell, radix literals are parsed as `u64` and reinterpreted, so
/// `0xffffffffffffffff` is `-1`.
pub fn parse_int(text: &str) -> Option<i64> {
    // Most words asked about are no number: an int has a digit.
    if !text.bytes().any(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let text = strip_underscores(text);
    alt((
        rest.try_map(str::parse::<i64>),
        preceded("0x", radix_digits(16)),
        preceded("0o", radix_digits(8)),
        preceded("0b", radix_digits(2)),
    ))
    .parse(&text)
    .ok()
}

/// The rest of the input as the digits of an unsigned number in `radix`,
/// reinterpreted as an `i64`.
fn radix_digits<'i>(radix: u32) -> impl Parser<&'i str, i64, EmptyError> {
    rest.try_map(move |digits| u64::from_str_radix(digits, radix)).map(|value| value as i64)
}

/// The radix announced by a `0x`, `0o` or `0b` prefix, if any.
pub fn radix_prefix(text: &str) -> Option<u32> {
    match text.as_bytes() {
        [b'0', b'x', ..] => Some(16),
        [b'0', b'o', ..] => Some(8),
        [b'0', b'b', ..] => Some(2),
        _ => None,
    }
}

/// A float literal (nu's `parse_float`): everything Rust's `f64::from_str`
/// accepts (including `inf`, `NaN`, `1e5`, `.5`), with `_` separators.
pub fn parse_float(text: &str) -> Option<f64> {
    // Most words asked about are no number: a float has a digit, unless it is `inf`,
    // `infinity` or `NaN` (in any case, signed, with separators).
    let special_float_byte =
        |byte: u8| matches!(byte.to_ascii_lowercase(), b'+' | b'-' | b'_' | b'i' | b'n' | b'f' | b'a' | b't' | b'y');
    if !text.bytes().any(|byte| byte.is_ascii_digit()) && !text.bytes().all(special_float_byte) {
        return None;
    }
    let text = strip_underscores(text);
    if text.is_empty() {
        return None;
    }
    text.parse::<f64>().ok()
}

/// An int or a float (nu's `parse_number`).
pub fn parse_number<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Expression<'a>> {
    let text = working_set.get_span_contents(span);
    match (parse_int(text), parse_float(text)) {
        (Some(int), _) => Ok(Expression::new(Expr::Int(int), span)),
        (None, Some(float)) => Ok(Expression::new(Expr::Float(float), span)),
        (None, None) => Err(cut(Diagnostic::expected("number", span))),
    }
}

/// `true` if `text` could start a unit literal: a digit, `.digit` or `-digit`.
fn unit_literal_start(text: &[u8]) -> bool {
    text.len() >= 2
        && (text[0].is_ascii_digit()
            || (text[0] == b'.' && text[1].is_ascii_digit())
            || (text[0] == b'-' && text[1].is_ascii_digit()))
}

/// The filesize units in upper case. [`parse_unit_value`] takes the first one
/// the text ends with, so `B`, the end of every other unit, comes last.
const FILESIZE_UNITS: &[(&str, FilesizeUnit)] = &[
    ("KB", FilesizeUnit::KB),
    ("MB", FilesizeUnit::MB),
    ("GB", FilesizeUnit::GB),
    ("TB", FilesizeUnit::TB),
    ("PB", FilesizeUnit::PB),
    ("EB", FilesizeUnit::EB),
    ("KIB", FilesizeUnit::KiB),
    ("MIB", FilesizeUnit::MiB),
    ("GIB", FilesizeUnit::GiB),
    ("TIB", FilesizeUnit::TiB),
    ("PIB", FilesizeUnit::PiB),
    ("EIB", FilesizeUnit::EiB),
    ("B", FilesizeUnit::B),
];

/// The duration units; `µs` is spelled with the micro sign or the Greek mu.
const DURATION_UNITS: &[(&str, DurationUnit)] = &[
    ("ns", DurationUnit::Nanosecond),
    ("us", DurationUnit::Microsecond),
    ("\u{00B5}s", DurationUnit::Microsecond),
    ("\u{03BC}s", DurationUnit::Microsecond),
    ("ms", DurationUnit::Millisecond),
    ("sec", DurationUnit::Second),
    ("min", DurationUnit::Minute),
    ("hr", DurationUnit::Hour),
    ("day", DurationUnit::Day),
    ("wk", DurationUnit::Week),
];

/// A number followed by one of `units` (nu's `parse_unit_value`), compared
/// in upper case when `uppercase` is set. `Err` when the text ends in a unit
/// but what comes before it is not a number.
fn parse_unit_value<'u, U: Copy>(
    text: &str,
    units: &'u [(&'u str, U)],
    uppercase: bool,
) -> Option<Result<(f64, U), &'static str>> {
    if !unit_literal_start(text.as_bytes()) {
        return None;
    }
    let normalized: Cow<'_, str> = if uppercase { Cow::Owned(text.to_ascii_uppercase()) } else { Cow::Borrowed(text) };
    let (name, unit) = units.iter().find(|(name, _)| normalized.ends_with(name))?;
    // Like nu, the check for a variable before the unit (`$x..1kb`) comes
    // after the underscores are stripped, so `42m$_b` is no filesize either.
    let number = strip_underscores(&text[..text.len() - name.len()]);
    if number.ends_with('$') {
        return None;
    }
    match number.parse::<f64>() {
        Ok(value) => Some(Ok((value, *unit))),
        Err(_) => Some(Err("value must be a number")),
    }
}

/// A filesize literal such as `10kb` or `1.5MiB` (nu's `parse_filesize`;
/// units are case-insensitive).
pub fn parse_filesize(text: &str) -> Option<Result<Filesize, &'static str>> {
    // `0x1b` would otherwise look like `0x1` bytes.
    if text.starts_with("0x") {
        return None;
    }
    parse_unit_value(text, FILESIZE_UNITS, true).map(|result| result.map(|(value, unit)| Filesize { value, unit }))
}

/// A duration literal such as `1sec` or `2.5hr` (nu's `parse_duration`;
/// units are case-sensitive).
pub fn parse_duration(text: &str) -> Option<Result<Duration, &'static str>> {
    parse_unit_value(text, DURATION_UNITS, false).map(|result| result.map(|(value, unit)| Duration { value, unit }))
}

/// Exactly `count` ASCII digits, as a number.
fn digits<'i>(count: usize) -> impl Parser<&'i str, u32, EmptyError> {
    take_while(count..=count, |digit: char| digit.is_ascii_digit()).parse_to::<u32>()
}

/// The number of days in a month of the proleptic Gregorian calendar.
fn days_in_month(year: u32, month: u32) -> u32 {
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// `true` if `text` is a date/time literal Nushell would accept, i.e. what
/// chrono's RFC 3339 parser takes (nu's `parse_datetime`):
///
/// ```text
/// datetime = date [ time [ offset ] ]
/// date     = YYYY "-" MM "-" DD                      a real calendar date
/// time     = ("T" | "t") hh ":" mm ":" ss [ "." digits ]   seconds up to 60 (a leap second)
/// offset   = "Z" | "z" | ("+" | "-") hh ":" mm
/// ```
pub fn is_datetime(text: &str) -> bool {
    (date, opt((time, opt(offset)))).parse(text).is_ok()
}

/// The `date` of [`is_datetime`]: `YYYY-MM-DD`, a day that exists in that month.
fn date(input: &mut &str) -> winnow::Result<(), EmptyError> {
    let year = digits(4).parse_next(input)?;
    let month = preceded('-', digits(2).verify(|month| (1..=12).contains(month))).parse_next(input)?;
    preceded('-', digits(2).verify(|day| (1..=days_in_month(year, month)).contains(day))).void().parse_next(input)
}

/// The `time` of [`is_datetime`]: `Thh:mm:ss` with optional fractional seconds.
fn time(input: &mut &str) -> winnow::Result<(), EmptyError> {
    (
        one_of(['T', 't']),
        digits(2).verify(|hour| *hour < 24),
        ':',
        digits(2).verify(|minute| *minute < 60),
        ':',
        digits(2).verify(|second| *second < 61),
        opt(preceded('.', digit1)),
    )
        .void()
        .parse_next(input)
}

/// The `offset` of [`is_datetime`]: `Z`, or `+hh:mm` / `-hh:mm` from UTC.
fn offset(input: &mut &str) -> winnow::Result<(), EmptyError> {
    alt((
        one_of(['Z', 'z']).void(),
        (one_of(['+', '-']), digits(2).verify(|hour| *hour < 24), ':', digits(2).verify(|minute| *minute < 60)).void(),
    ))
    .parse_next(input)
}

/// Whether a `0x[...]`/`0o[...]`/`0b[...]` word makes a command line a math
/// expression: like nu, it does unless the brackets hold a pipe, redirection
/// or assignment token (`0b[1|2]` is then a command name). A bracket interior
/// that does not lex counts as binary, so [`parse_binary`] reports the error.
pub fn looks_like_binary(text: &str) -> bool {
    let Some(prefix) = ["0x[", "0o[", "0b["].into_iter().find(|prefix| text.starts_with(prefix)) else { return false };
    let Some(inner) = text[prefix.len()..].strip_suffix(']') else { return false };
    match lex(inner, 0, LexOptions::BINARY) {
        Ok(tokens) => tokens.iter().all(|token| {
            matches!(
                token.contents,
                TokenContents::Item
                    | TokenContents::Eof
                    | TokenContents::Comment
                    | TokenContents::Semicolon
                    | TokenContents::Eol
            )
        }),
        Err(_) => true,
    }
}

/// `0x[...]`, `0o[...]` or `0b[...]` (nu's `parse_binary`). `None` if the
/// item does not look like a binary literal at all.
pub fn parse_binary<'a>(working_set: &WorkingSet<'a>, span: Span) -> Option<ParseResult<Expression<'a>>> {
    let text = working_set.get_span_contents(span);
    let (radix, digits_per_byte, prefix) = match text.as_bytes() {
        // Only a bracketed literal that closes is a binary; `0x[13]=` is a bare word in nu.
        [.., last] if *last != b']' => return None,
        [b'0', b'x', b'[', ..] => (16, 2, "0x["),
        [b'0', b'o', b'[', ..] => (8, 3, "0o["),
        [b'0', b'b', b'[', ..] => (2, 8, "0b["),
        _ => return None,
    };
    Some(parse_binary_with_base(working_set, span, text, radix, digits_per_byte, prefix))
}

/// The bytes of a binary literal in `radix` (nu's `parse_binary_with_base`),
/// whose digits may be split by whitespace, commas and comments.
fn parse_binary_with_base<'a>(
    working_set: &WorkingSet<'a>,
    span: Span,
    text: &str,
    radix: u32,
    digits_per_byte: usize,
    prefix: &str,
) -> ParseResult<Expression<'a>> {
    let Some(inner) = text.strip_prefix(prefix).and_then(|token| token.strip_suffix(']')) else {
        return Err(cut(Diagnostic::expected("binary literal", span)));
    };
    let inner_span = Span::new(span.start + prefix.len(), span.end - 1);
    let tokens = lex(inner, inner_span.start, LexOptions::BINARY).map_err(cut)?;
    let mut digits = String::new();
    for token in &tokens {
        match token.contents {
            TokenContents::Item => digits.push_str(working_set.get_span_contents(token.span)),
            TokenContents::Eof | TokenContents::Comment | TokenContents::Semicolon | TokenContents::Eol => {}
            _ => return Err(cut(Diagnostic::expected("binary digits", token.span))),
        }
    }
    if let Some(bad) = digits.chars().find(|digit| !digit.is_digit(radix)) {
        return Err(cut(Diagnostic::new(
            ErrorKind::InvalidLiteral {
                kind: "binary",
                message: format!("`{bad}` is not a valid digit for radix {radix}"),
            },
            span,
        )));
    }
    // Like nu, a short first byte is padded on the left: `0x[abc]` is `0x[0a bc]`.
    let padding = (digits_per_byte - digits.len() % digits_per_byte) % digits_per_byte;
    let padded = format!("{}{}", "0".repeat(padding), digits);
    let mut bytes = Vec::with_capacity(padded.len() / digits_per_byte);
    for chunk in padded.as_bytes().chunks(digits_per_byte) {
        let byte_digits = std::str::from_utf8(chunk).expect("ascii digits");
        match u8::from_str_radix(byte_digits, radix) {
            Ok(byte) => bytes.push(byte),
            Err(_) => {
                return Err(cut(Diagnostic::new(
                    ErrorKind::InvalidLiteral {
                        kind: "binary",
                        message: format!("`{byte_digits}` does not fit in a byte"),
                    },
                    span,
                )));
            }
        }
    }
    Ok(Expression::new(Expr::Binary(BinaryLiteral { radix, bytes }), span))
}

/// Decode the escape sequences of a double-quoted string body (nu's
/// `unescape_string`):
///
/// ```text
/// body   = { text | escape }
/// escape = "\\" ( one of `"'\\/(){}$^#|~` or a space   the character itself
///               | "a" | "b" | "e" | "f" | "n" | "r" | "t" | "0"
///               | "x" hex hex                          one raw byte
///               | "u{" hex{1,6} "}" )                  a code point
/// ```
///
/// Like nu, `\xHH` contributes a raw byte and the result must be valid UTF-8
/// as a whole: `"\xC3\xA9"` is `é`, `"\xC3"` is an error. `base` is the
/// absolute offset of `text`, used for error spans.
fn unescape_string(text: &str, base: usize) -> Result<Cow<'_, str>, Diagnostic> {
    if !text.contains('\\') {
        return Ok(Cow::Borrowed(text));
    }
    let decoded = repeat(0.., alt((take_till(1.., '\\').map(Unescaped::Text), escape_sequence)))
        .fold(
            || Vec::with_capacity(text.len()),
            |mut bytes: Vec<u8>, piece| {
                match piece {
                    Unescaped::Text(text) => bytes.extend_from_slice(text.as_bytes()),
                    Unescaped::Byte(byte) => bytes.push(byte),
                    Unescaped::Char(character) => {
                        bytes.extend_from_slice(character.encode_utf8(&mut [0; 4]).as_bytes())
                    }
                }
                bytes
            },
        )
        .parse_next(&mut input(text, base))
        .map_err(into_diagnostic)?;
    String::from_utf8(decoded).map(Cow::Owned).map_err(|_| {
        invalid_string(
            "the string is not valid UTF-8 after decoding its escapes".into(),
            Span::new(base, base + text.len()),
        )
        .with_help("`\\xHH` escapes must form UTF-8 sequences; use `\\u{...}` for a code point")
    })
}

/// A piece of a decoded string body.
enum Unescaped<'t> {
    /// Text without escapes, taken as it is.
    Text(&'t str),
    /// One byte: an escaped character, or a `\xHH` raw byte.
    Byte(u8),
    /// A `\u{...}` code point.
    Char(char),
}

/// One escape sequence, starting at its backslash.
fn escape_sequence<'t>(input: &mut Input<'t>) -> ParseResult<Unescaped<'t>> {
    let start = pos(input);
    let end = rest_span(input).end;
    '\\'.parse_next(input)?;
    let Some(escaped) = opt(any).parse_next(input)? else {
        return Err(cut(invalid_string("incomplete escape sequence after `\\`".into(), Span::new(start, end))));
    };
    let byte = match escaped {
        '"' | '\'' | '\\' | '/' | '(' | ')' | '{' | '}' | '$' | '^' | '#' | '|' | '~' | ' ' => escaped as u8,
        'a' => 0x07,
        'b' => 0x08,
        'e' => 0x1b,
        'f' => 0x0c,
        'n' => b'\n',
        'r' => b'\r',
        't' => b'\t',
        '0' => 0,
        'x' => {
            let hex: ParseResult<&str> = take_while(2..=2, |digit: char| digit.is_ascii_hexdigit()).parse_next(input);
            let Ok(hex) = hex else {
                let message = "incomplete hex escape '\\xHH', expected 2 hex digits";
                return Err(cut(invalid_string(message.into(), Span::new(start, end.min(start + 4)))));
            };
            u8::from_str_radix(hex, 16).expect("two hex digits")
        }
        'u' => {
            let braced: ParseResult<&str> = delimited('{', take_till(0.., '}'), '}').parse_next(input);
            let Ok(hex) = braced else {
                let message = "incomplete unicode escape '\\u{...}', missing closing '}'";
                return Err(cut(invalid_string(message.into(), Span::new(start, end.min(start + 3)))));
            };
            let code_point = u32::from_str_radix(hex, 16).ok().filter(|_| (1..=6).contains(&hex.len()));
            return match code_point.and_then(char::from_u32) {
                Some(character) => Ok(Unescaped::Char(character)),
                None => Err(cut(invalid_string(
                    format!("invalid unicode escape '\\u{{{hex}}}', must be 1-6 hex digits, max codepoint 0x10FFFF"),
                    Span::new(start, start + 3 + hex.len()),
                ))),
            };
        }
        other => {
            return Err(cut(invalid_string(
                format!("unrecognized escape sequence `\\{other}`"),
                Span::new(start, start + 1 + other.len_utf8()),
            )));
        }
    };
    Ok(Unescaped::Byte(byte))
}

/// An invalid string literal.
fn invalid_string(message: String, span: Span) -> Diagnostic {
    Diagnostic::new(ErrorKind::InvalidLiteral { kind: "string", message }, span)
}

/// A raw string `r#'...'#` (nu's `parse_raw_string`): the text between the
/// quotes, taken as it is.
///
/// ```text
/// raw-string = "r" "#"{n} "'" text "'" "#"{n}        n from 1 to 255
/// ```
///
/// The text runs from the quote after the opening hashes to the quote before
/// as many hashes at the end of the item; anything else is an unclosed quote
/// (`r#'a'#b`).
pub fn parse_raw_string<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Expression<'a>> {
    let text = working_set.get_span_contents(span);
    let Some(after_r) = text.strip_prefix('r') else {
        return Err(cut(Diagnostic::expected("raw string", span)));
    };
    let hashes = after_r.bytes().take_while(|byte| *byte == b'#').count();
    if hashes == 0 || hashes > u8::MAX as usize {
        return Err(cut(Diagnostic::expected("`#` after `r` in raw string", span)));
    }
    let body_start = 1 + hashes;
    let body_end = text.len().checked_sub(hashes).filter(|body_end| *body_end > body_start + 1);
    let Some(body_end) = body_end else {
        return Err(cut(Diagnostic::new(ErrorKind::Unclosed { delimiter: "'", open: span }, span.past())));
    };
    if !text[body_end..].bytes().all(|byte| byte == b'#')
        || text.as_bytes()[body_start] != b'\''
        || text.as_bytes()[body_end - 1] != b'\''
    {
        return Err(cut(Diagnostic::new(ErrorKind::Unclosed { delimiter: "'", open: span }, span.past())));
    }
    let value = &text[body_start + 1..body_end - 1];
    Ok(Expression::new(
        Expr::String(StringLiteral { value: Cow::Borrowed(value), quote: Quote::Raw(hashes as u8) }),
        span,
    ))
}

/// `true` for a bare word that Nushell treats as an interpolation because it
/// contains `(`: `foo(1 + 1)bar`.
fn is_bare_string_interpolation(text: &str) -> bool {
    !text.starts_with(['\'', '"', '`']) && text.contains('(')
}

/// A string item (nu's `parse_string`): quoted (`"`, `'`, `` ` ``), raw, bare, or a bare interpolation.
pub fn parse_string<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Expression<'a>> {
    let text = working_set.get_span_contents(span);
    match text {
        "" => Err(cut(Diagnostic::expected("string", span))),
        _ if text.starts_with("r#") => parse_raw_string(working_set, span),
        _ if is_bare_string_interpolation(text) => parse_string_interpolation(working_set, span),
        _ => Ok(Expression::new(Expr::String(parse_string_literal(working_set, span)?), span)),
    }
}

/// The literal of a quoted, backtick or bare string item; a bare-word
/// interpolation is *not* handled here.
pub fn parse_string_literal<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<StringLiteral<'a>> {
    let text = working_set.get_span_contents(span);
    match text.as_bytes().first() {
        Some(b'"') => {
            let body = quoted_string_body(working_set, span, b'"')?;
            Ok(StringLiteral { value: unescape_string(body, span.start + 1).map_err(cut)?, quote: Quote::Double })
        }
        Some(b'\'') => Ok(StringLiteral {
            value: Cow::Borrowed(quoted_string_body(working_set, span, b'\'')?),
            quote: Quote::Single,
        }),
        // Backticks are trimmed only when the item both starts and ends with
        // one; `` `a`b `` is the bare word `` `a`b ``, as in nu.
        Some(b'`') if text.len() >= 2 && text.ends_with('`') => {
            Ok(StringLiteral { value: Cow::Borrowed(&text[1..text.len() - 1]), quote: Quote::Backtick })
        }
        _ => Ok(StringLiteral::bare(text)),
    }
}

/// The text between the quotes of a quoted item.
///
/// Like Nushell, the *last* quote character in the item must be its final
/// byte (`"a"b` is an error) but quotes in between are kept as text, so
/// `"a"b"c"` is the string `a"b"c`.
fn quoted_string_body<'a>(working_set: &WorkingSet<'a>, span: Span, quote: u8) -> ParseResult<&'a str> {
    let text = working_set.get_span_contents(span);
    let bytes = text.as_bytes();
    match bytes.iter().rposition(|byte| *byte == quote) {
        Some(0) | None => Err(cut(Diagnostic::new(
            ErrorKind::Unclosed { delimiter: quote_str(quote), open: Span::new(span.start, span.start + 1) },
            span.past(),
        ))),
        Some(last) if last + 1 != bytes.len() => {
            Err(cut(Diagnostic::new(ErrorKind::ExtraTokens, Span::new(span.start + last + 1, span.end))
                .with_help("invalid characters after the closing quote; quote the whole string or remove them")))
        }
        Some(last) => Ok(&text[1..last]),
    }
}

/// A quote character as a string, for diagnostics.
pub fn quote_str(quote: u8) -> &'static str {
    match quote {
        b'"' => "\"",
        b'\'' => "'",
        _ => "`",
    }
}

/// `$"..."`, `$'...'` or a bare interpolation `foo(...)` (nu's
/// `parse_string_interpolation`).
fn parse_string_interpolation<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Expression<'a>> {
    let text = working_set.get_span_contents(span);
    let (quote, body) = match text.as_bytes() {
        [b'$', q @ (b'"' | b'\''), ..] => {
            let body = quoted_string_body(working_set, Span::new(span.start + 1, span.end), *q)?;
            let body = Span::new(span.start + 2, span.start + 2 + body.len());
            (if *q == b'"' { Quote::Double } else { Quote::Single }, body)
        }
        _ => (Quote::Bare, span),
    };
    let parts = parse_interpolation_parts(working_set, body, quote)?;
    Ok(Expression::new(Expr::StringInterpolation(StringInterpolation { quote, parts }), span))
}

/// Split the body of an interpolated string into literal text and `( ... )`
/// subexpressions, using the same delimiter rules as the lexer (the loop of
/// nu's `parse_string_interpolation`).
///
/// ```text
/// body = { text | "(" subexpression ")" }
/// ```
///
/// A byte scan rather than combinators, because it must find the `)` the
/// lexer found: it tracks quotes and nested parentheses with
/// [`interp_subexpr_step`], the step the lexer takes inside `$"..."`. In
/// double quotes `\(` is text, and the text parts are unescaped.
fn parse_interpolation_parts<'a>(
    working_set: &WorkingSet<'a>,
    body: Span,
    quote: Quote,
) -> ParseResult<Vec<InterpolationPart<'a>>> {
    let text = working_set.get_span_contents(body);
    let bytes = text.as_bytes();
    let double = quote == Quote::Double;
    let mut parts = Vec::new();
    let mut index = 0;
    let mut part_start = 0;
    let mut backslashes = 0usize;
    let mut stack: Vec<(u8, usize)> = Vec::new();
    let mut in_expr = false;

    let text_part = |start: usize, end: usize| -> ParseResult<Option<InterpolationPart<'a>>> {
        if start >= end {
            return Ok(None);
        }
        let span = Span::new(body.start + start, body.start + end);
        let lite_command = &text[start..end];
        let value =
            if double { unescape_string(lite_command, span.start).map_err(cut)? } else { Cow::Borrowed(lite_command) };
        Ok(Some(InterpolationPart::Text { span, value }))
    };

    while index < bytes.len() {
        let c = bytes[index];
        if !in_expr {
            let preceding = backslashes;
            backslashes = if c == b'\\' { preceding + 1 } else { 0 };
            // In double quotes `\(` is a literal parenthesis.
            if c == b'(' && (!double || preceding.is_multiple_of(2)) {
                parts.extend(text_part(part_start, index)?);
                in_expr = true;
                part_start = index;
                stack.push((b')', index));
            }
            index += 1;
            continue;
        }
        if interp_subexpr_step(&mut stack, c, index) && index + 1 < bytes.len() {
            index += 2;
            continue;
        }
        if c == b')' && stack.is_empty() {
            let span = Span::new(body.start + part_start, body.start + index + 1);
            let expression = parse_paren_expr(working_set, span, ExpectedShape::Any)?;
            parts.push(InterpolationPart::Expression(Box::new(expression)));
            in_expr = false;
            part_start = index + 1;
        }
        index += 1;
    }
    if in_expr {
        return Err(cut(Diagnostic::new(
            ErrorKind::Unclosed {
                delimiter: ")",
                open: Span::new(body.start + part_start, body.start + part_start + 1),
            },
            body.past(),
        )));
    }
    parts.extend(text_part(part_start, bytes.len())?);
    Ok(parts)
}

/// `$name` (nu's `parse_variable_expr`).
fn parse_variable_expr<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Expression<'a>> {
    let text = working_set.get_span_contents(span);
    let name = text.strip_prefix('$').unwrap_or(text);
    if !is_identifier(name) {
        return Err(cut(Diagnostic::expected("valid variable name", span)
            .with_help("variable names may not contain `.[({+-*^%/=!<>&|`")));
    }
    Ok(Expression::new(Expr::Var(Var { name }), span))
}

/// An item starting with `$` (nu's `parse_dollar_expr`): an interpolation, a
/// cell-path literal, a range, or a variable with an optional cell path.
pub fn parse_dollar_expr<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Expression<'a>> {
    let text = working_set.get_span_contents(span);
    match text.as_bytes() {
        [b'$', b'"' | b'\'', ..] => parse_string_interpolation(working_set, span),
        [b'$', b'.', ..] => {
            // `$.` alone is the empty cell path (the identity).
            let members_span = Span::new(span.start + 2, span.end);
            let items = lex_cell_path(working_set, members_span)?;
            let members = parse_cell_path(&mut Tokens::new(working_set, &items, span.end), false)?;
            Ok(Expression::new(Expr::CellPath(CellPath { members }), span))
        }
        _ if is_range_syntax(text) => parse_range(working_set, span),
        _ => parse_full_cell_path(working_set, span, false),
    }
}

/// An item starting with `(` (nu's `parse_paren_expr`): a range, a
/// signature, or a subexpression with an optional cell path.
pub fn parse_paren_expr<'a>(
    working_set: &WorkingSet<'a>,
    span: Span,
    shape: ExpectedShape<'_, 'a>,
) -> ParseResult<Expression<'a>> {
    match shape {
        _ if is_range_syntax(working_set.get_span_contents(span)) => parse_range(working_set, span),
        ExpectedShape::Signature => Ok(Expression::new(Expr::Garbage, span)),
        _ => parse_full_cell_path(working_set, span, false),
    }
}

/// A cell-path literal without the `$.`, as the `cell-path` shape reads it:
/// `a.b.0` (nu's `parse_simple_cell_path`).
pub fn parse_simple_cell_path<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Expression<'a>> {
    let items = lex_cell_path(working_set, span)?;
    let members = parse_cell_path(&mut Tokens::new(working_set, &items, span.end), false)?;
    Ok(Expression::new(Expr::CellPath(CellPath { members }), span))
}

/// A head (`$var`, `(...)`, `[...]`, `{...}`) followed by `.member`
/// accesses (nu's `parse_full_cell_path`): `$x.a.0`, `(ls).name?`, `{a: 1}.a`.
///
/// With `implicit` set, a bare head is taken as a column of the row variable
/// `$it` (used by `where` row conditions).
pub fn parse_full_cell_path<'a>(
    working_set: &WorkingSet<'a>,
    span: Span,
    implicit: bool,
) -> ParseResult<Expression<'a>> {
    let items = lex_cell_path(working_set, span)?;
    let Some(head_token) = items.first() else {
        return Err(cut(Diagnostic::expected("value", span)));
    };
    let head_text = working_set.get_span_contents(head_token.span);
    let (head, member_tokens) = match head_text.as_bytes() {
        // `(pwd)/x` and `(a)/b/(c)`: not a subexpression head but a bare interpolation.
        [b'(', ..] if working_set.group_end(head_token.span) != Some(head_text.len() - 1) => {
            return parse_string_interpolation(working_set, span);
        }
        [b'(', ..] => (parse_subexpression(working_set, head_token.span)?, &items[1..]),
        [b'[', ..] => (parse_list_expression(working_set, head_token.span)?, &items[1..]),
        [b'{', ..] => (parse_record(working_set, head_token.span)?, &items[1..]),
        [b'$', ..] => (parse_variable_expr(working_set, head_token.span)?, &items[1..]),
        _ if implicit => (Expression::new(Expr::Var(Var { name: "it" }), Span::point(span.start)), &items[..]),
        _ => return Err(cut(Diagnostic::expected("variable or subexpression", head_token.span))),
    };
    let members = parse_cell_path(&mut Tokens::new(working_set, member_tokens, span.end), !implicit)?;
    // A head alone is just the head; a trailing `.` (`$x.`) keeps the path, with the span of the
    // whole item, as in nu (a completer reads the `.` as the start of a member).
    if members.is_empty() && !implicit && head.span == span {
        return Ok(head);
    }
    Ok(Expression::new(
        Expr::FullCellPath(FullCellPath { head: Box::new(head), implicit_head: implicit, tail: members }),
        span,
    ))
}

/// The members of a cell path (nu's `parse_cell_path`), from the tokens of
/// an item lexed with [`LexOptions::CELL_PATH`], in which `.`, `?` and `!` are
/// items of their own:
///
/// ```text
/// cell-path = [ member ] { "." [ member ] }     (the first member only without a head)
/// member    = item { "?" | "!" }                (each modifier at most once)
/// ```
///
/// A trailing `.` is accepted, as in Nushell.
fn parse_cell_path<'a>(tokens: &mut Tokens<'_, 'a>, expect_dot: bool) -> ParseResult<Vec<PathMember<'a>>> {
    let mut first = match expect_dot {
        true => None,
        false => opt(path_member).parse_next(tokens)?,
    };
    // The member before the first `.`, if any, seeds the list.
    let members = repeat(0.., preceded(keyword("."), opt(path_member)))
        .fold(
            || Vec::from_iter(first.take()),
            |mut members, member: Option<PathMember<'a>>| {
                members.extend(member);
                members
            },
        )
        .parse_next(tokens)?;
    // Only a head's tail can get here with tokens left: `$x?`.
    match tokens.peek_token() {
        Some(token) => Err(cut(Diagnostic::expected("`.`", token.span))),
        None => Ok(members),
    }
}

/// One member, its `?`/`!` modifiers, and a check that a `.` or the end follows.
fn path_member<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<PathMember<'a>> {
    let token = item(tokens)?;
    let kind = parse_path_member_kind(tokens.working_set, token.span)?;
    let mut member = PathMember { span: token.span, kind, optional: false, case_insensitive: false };
    while let Some(modifier) = opt(alt((keyword("?"), keyword("!")))).parse_next(tokens)? {
        match (tokens.text(&modifier), member.optional, member.case_insensitive) {
            ("!", _, false) => member.case_insensitive = true,
            ("?", false, _) => member.optional = true,
            _ => return Err(expected_after_path_member(&member, modifier.span)),
        }
        member.span = member.span.merge(modifier.span);
    }
    match tokens.peek_token() {
        Some(next) if tokens.text(next) != "." => Err(expected_after_path_member(&member, next.span)),
        _ => Ok(member),
    }
}

/// A row index, or a column name.
fn parse_path_member_kind<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<PathMemberKind<'a>> {
    let text = working_set.get_span_contents(span);
    match parse_int(text) {
        Some(index) if index < 0 => Err(cut(Diagnostic::new(
            ErrorKind::InvalidLiteral { kind: "cell path", message: "negative index is not supported".into() },
            span,
        ))),
        Some(index) => Ok(PathMemberKind::Int(index as usize)),
        // nu parses the member as a string, and a bare word with a `(` in it
        // is an interpolation, not a string: `$x.a(b)` fails.
        None if is_bare_string_interpolation(text) => {
            Err(cut(Diagnostic::expected("string", span).with_help("a cell-path member with `(` in it must be quoted")))
        }
        None => Ok(PathMemberKind::String(parse_string_literal(working_set, span)?.value)),
    }
}

/// What may follow a member, given the modifiers it already has.
fn expected_after_path_member(member: &PathMember<'_>, span: Span) -> ErrMode<ParseFailure> {
    let what = match (member.optional, member.case_insensitive) {
        (false, false) => "`.`, `?` or `!`",
        (true, false) => "`.` or `!`",
        (false, true) => "`.` or `?`",
        (true, true) => "`.`",
    };
    cut(Diagnostic::expected(what, span))
}

/// The items of `span` lexed as a cell path, with `.`, `?` and `!` split off:
/// `$x.a?.0` gives `$x` `.` `a` `?` `.` `0`. Like nu, every token counts, so
/// an `=` or an `o>` is a member or a misplaced one (`$x.a?=` fails), never
/// dropped.
fn lex_cell_path(working_set: &WorkingSet<'_>, span: Span) -> ParseResult<Vec<Token>> {
    // Most cell paths are a head alone: `$name`, or a group whose closing bracket ends the
    // text. Either is one item, which lexing would only confirm. The `$name` bytes are ones
    // the cell-path lexer never splits at, valid in a name or not (`$a-b`).
    let text = working_set.get_span_contents(span);
    let head_alone = match text.as_bytes() {
        [b'$', name @ ..] => name.iter().all(|&byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')),
        [b'(' | b'[' | b'{', ..] => working_set.group_end(span) == Some(text.len() - 1),
        _ => false,
    };
    if head_alone {
        return Ok(vec![Token { contents: TokenContents::Item, span }]);
    }
    let mut lexed = working_set.lex(span, LexOptions::CELL_PATH).map_err(cut)?;
    lexed.retain(|token| !matches!(token.contents, TokenContents::Eof | TokenContents::Eol | TokenContents::Comment));
    for token in &mut lexed {
        token.contents = TokenContents::Item;
    }
    Ok(lexed)
}

/// Positions of the range operators (`..`) at parenthesis depth zero:
/// `(Some(next), operator)` for `a..b..c`, `(None, operator)` for `a..b`.
fn find_range_operators(text: &str) -> Option<(Option<usize>, usize)> {
    let mut depth = 0i32;
    let mut positions = Vec::with_capacity(2);
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b'.' if depth == 0 && bytes.get(index + 1) == Some(&b'.') => {
                positions.push(index);
                index += 2;
                continue;
            }
            _ => {}
        }
        index += 1;
    }
    match positions.as_slice() {
        [operator] => Some((None, *operator)),
        [next, operator] => Some((Some(*next), *operator)),
        _ => None,
    }
}

/// A range bound must be something `parse_value(Number)` accepts: a number,
/// a `$` expression or a parenthesised subexpression, which may carry a cell
/// path (`(ls).0..5`).
fn is_range_bound(text: &str) -> bool {
    parse_int(text).is_some()
        || parse_float(text).is_some()
        || text.starts_with('$')
        || (text.starts_with('(') && group_end(text).is_some())
}

/// `true` if `text` has the shape of a range: `from..to`, `from..<to`,
/// `from..=to`, `from..next..to`, `..to`, `from..`, with every present bound
/// number-like. Decided without parsing, so callers can fall through to the
/// next literal kind (`cd ..`, `a..b`) when it is not.
pub fn is_range_syntax(text: &str) -> bool {
    // Most items hold no `..`; answer those without a call.
    has_range_operator(text)
        && range_bounds(text).is_some_and(|(from, next, to)| {
            (from.is_empty() || is_range_bound(from))
                && next.is_none_or(|next| !next.is_empty() && is_range_bound(next))
                && (to.is_empty() || is_range_bound(to))
        })
}

/// Whether `text` holds `..` anywhere. The search goes from `.` to `.` (a
/// `memchr`), since it also runs over whole subexpressions.
#[inline]
fn has_range_operator(text: &str) -> bool {
    let mut rest = text;
    while let Some(dot) = rest.find('.') {
        rest = &rest[dot + 1..];
        if rest.starts_with('.') {
            return true;
        }
    }
    false
}

/// Whether a command head is a range (nu's `is_math_expression_like`, which
/// asks `parse_range` for a range without an error): [`is_range_syntax`] with
/// no bare `$` bound. As a head, `..$` is then an external command; as a value
/// it is still a range, whose `$` is an incomplete variable.
#[inline]
pub fn is_range_head(text: &str) -> bool {
    // A `$` bound must name a variable and a cell path on it (`..$x.a`);
    // `..$`, `1..$s=`, `1..$s!` and `..$x.c!!` are no range for nu, whose
    // value parser fails on them, nor is `..$a:` (no variable has a `:` in
    // its name: `let` reads `name: type`).
    let bound_parses = |bound: &str| match bound.strip_prefix('$') {
        Some(rest) => {
            let name_end = rest.find(['.', '?', '!']).unwrap_or(rest.len());
            let name = &rest[..name_end];
            is_identifier(name) && !name.contains(':') && is_cell_path_tail(&rest[name_end..])
        }
        None => true,
    };
    is_range_syntax(text)
        && range_bounds(text)
            .is_some_and(|(from, next, to)| bound_parses(from) && next.is_none_or(bound_parses) && bound_parses(to))
}

/// Whether `tail`, the text after a variable's name, is a cell path that
/// [`parse_full_cell_path`] accepts: `.member` accesses, each with at most one
/// `?` and one `!` after it (`$x.a?.b!`, not `$x.c!!`), no negative index and
/// no bare `(` in a member. A quoted member may hold any character.
fn is_cell_path_tail(tail: &str) -> bool {
    let mut rest = tail;
    while let Some(member) = rest.strip_prefix('.') {
        let end = match member.as_bytes().first() {
            Some(&quote @ (b'"' | b'\'' | b'`')) => match member[1..].find(char::from(quote)) {
                Some(close) => close + 2,
                None => return false,
            },
            _ => member.find(['.', '?', '!']).unwrap_or(member.len()),
        };
        let text = &member[..end];
        if parse_int(text).is_some_and(|index| index < 0) || is_bare_string_interpolation(text) {
            return false;
        }
        let (mut optional, mut case_insensitive) = (false, false);
        rest = &member[end..];
        loop {
            match rest.as_bytes().first() {
                Some(b'?') if !optional => optional = true,
                Some(b'!') if !case_insensitive => case_insensitive = true,
                _ => break,
            }
            rest = &rest[1..];
        }
    }
    rest.is_empty()
}

/// The `from`, `next` and `to` texts of a range-shaped `text` (empty for an
/// absent bound), or `None` when its operators do not make a range.
fn range_bounds(text: &str) -> Option<(&str, Option<&str>, &str)> {
    if !has_range_operator(text) || text.starts_with("...") {
        return None;
    }
    let (next_operator, operator) = find_range_operators(text)?;
    let operator_length =
        if text[operator..].starts_with("..<") || text[operator..].starts_with("..=") { 3 } else { 2 };
    // Only the last operator may be `..<` (`1..<2..3` is no range).
    if text.find("..<").is_some_and(|position| position != operator) {
        return None;
    }
    let from = &text[..next_operator.unwrap_or(operator)];
    let next = next_operator.map(|next_operator| &text[next_operator + 2..operator]);
    let to = &text[operator + operator_length..];
    if from.is_empty() && to.is_empty() {
        return None;
    }
    Some((from, next, to))
}

/// A range item (nu's `parse_range`); the caller has checked [`is_range_syntax`].
///
/// ```text
/// range = [ from ] [ ".." next ] ( ".." | "..=" | "..<" ) [ to ]     from or to present
/// ```
///
/// The operators are the `..`s outside parentheses (`(1..2).0..5` has one);
/// each bound is a value of the `number` shape.
pub fn parse_range<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Expression<'a>> {
    let text = working_set.get_span_contents(span);
    let Some((next_operator, operator)) = find_range_operators(text) else {
        return Err(cut(Diagnostic::expected("range", span)));
    };
    let (inclusion, operator_length) = match &text[operator..] {
        rest if rest.starts_with("..<") => (RangeInclusion::RightExclusive, 3),
        rest if rest.starts_with("..=") => (RangeInclusion::Inclusive, 3),
        _ => (RangeInclusion::Inclusive, 2),
    };
    let bound = |start: usize, end: usize| -> ParseResult<Option<Box<Expression<'a>>>> {
        if start >= end {
            return Ok(None);
        }
        let bound_span = Span::new(span.start + start, span.start + end);
        let bound = parse_value(working_set, bound_span, ExpectedShape::Number)?;
        // `(1)abc..5`: nu reads the bound as a bare interpolation, a string,
        // which the `..` operator then refuses.
        if let Expr::StringInterpolation(_) = bound.expr {
            return Err(cut(Diagnostic::message("the `..` operator does not work on a string", bound_span)
                .with_help("a range bound is a number, a variable or a subexpression")));
        }
        Ok(Some(Box::new(bound)))
    };
    let from = bound(0, next_operator.unwrap_or(operator))?;
    let next = match next_operator {
        Some(next_operator) => bound(next_operator + 2, operator)?,
        None => None,
    };
    let to = bound(operator + operator_length, text.len())?;
    Ok(Expression::new(
        Expr::Range(Range {
            from,
            next,
            to,
            operator: RangeOperator {
                inclusion,
                span: Span::new(span.start + operator, span.start + operator + operator_length),
                next_op_span: next_operator
                    .map(|next_operator| Span::new(span.start + next_operator, span.start + next_operator + 2)),
            },
        }),
        span,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ints() {
        assert_eq!(parse_int("42"), Some(42));
        assert_eq!(parse_int("-42"), Some(-42));
        assert_eq!(parse_int("+7"), Some(7));
        assert_eq!(parse_int("1_000_000"), Some(1_000_000));
        assert_eq!(parse_int("0xff"), Some(255));
        assert_eq!(parse_int("0o17"), Some(15));
        assert_eq!(parse_int("0b101"), Some(5));
        assert_eq!(parse_int("0xffffffffffffffff"), Some(-1));
        assert_eq!(parse_int("1.5"), None);
        assert_eq!(parse_int("abc"), None);
        assert_eq!(parse_int(""), None);
        assert_eq!(parse_int("99999999999999999999"), None);
    }

    #[test]
    fn floats() {
        assert_eq!(parse_float("1.5"), Some(1.5));
        assert_eq!(parse_float(".5"), Some(0.5));
        assert_eq!(parse_float("5."), Some(5.0));
        assert_eq!(parse_float("1e3"), Some(1000.0));
        assert_eq!(parse_float("1_0.5"), Some(10.5));
        assert_eq!(parse_float("inf"), Some(f64::INFINITY));
        assert!(parse_float("NaN").unwrap().is_nan());
        assert_eq!(parse_float("abc"), None);
    }

    #[test]
    fn units() {
        assert_eq!(parse_filesize("1kb").unwrap().unwrap(), Filesize { value: 1.0, unit: FilesizeUnit::KB });
        assert_eq!(parse_filesize("1.5MiB").unwrap().unwrap(), Filesize { value: 1.5, unit: FilesizeUnit::MiB });
        assert_eq!(parse_filesize("10B").unwrap().unwrap(), Filesize { value: 10.0, unit: FilesizeUnit::B });
        assert_eq!(parse_filesize("1b").unwrap().unwrap().unit, FilesizeUnit::B);
        assert!(parse_filesize("0x1b").is_none());
        assert!(parse_filesize("kb").is_none());
        assert!(parse_filesize("1_000kb").unwrap().is_ok());
        assert_eq!(parse_duration("5ns").unwrap().unwrap().unit, DurationUnit::Nanosecond);
        assert_eq!(parse_duration("1\u{00B5}s").unwrap().unwrap().unit, DurationUnit::Microsecond);
        assert_eq!(parse_duration("2.5hr").unwrap().unwrap(), Duration { value: 2.5, unit: DurationUnit::Hour });
        assert!(parse_duration("5NS").is_none());
        assert!(parse_duration("-3sec").unwrap().is_ok());
        assert!(parse_duration("1..2sec").unwrap().is_err());
    }

    #[test]
    fn datetimes() {
        assert!(is_datetime("2024-01-02"));
        assert!(is_datetime("2024-01-02T03:04:05"));
        assert!(is_datetime("2024-01-02T03:04:05.123Z"));
        assert!(is_datetime("2024-01-02T03:04:05+05:30"));
        assert!(!is_datetime("2024-13-02"));
        assert!(!is_datetime("2024-01-02T25:00:00"));
        assert!(!is_datetime("2024-01-02x"));
        assert!(!is_datetime("2024-01"));
    }

    #[test]
    fn escapes() {
        assert_eq!(unescape_string("plain", 0).unwrap(), "plain");
        assert!(matches!(unescape_string("plain", 0).unwrap(), Cow::Borrowed(_)));
        assert_eq!(unescape_string(r#"a\nb\t\"\\\("#, 0).unwrap(), "a\nb\t\"\\(");
        assert_eq!(unescape_string(r"\u{1F600}\x41\e", 0).unwrap(), "😀A\u{1b}");
        let err = unescape_string(r"a\qb", 10).unwrap_err();
        assert_eq!(err.span, Span::new(11, 13));
        assert!(unescape_string(r"\x4", 0).is_err());
        assert!(unescape_string(r"\u{110000}", 0).is_err());
        assert!(unescape_string(r"abc\", 0).is_err());
        // Hex escapes are bytes; the whole string must be UTF-8.
        assert_eq!(unescape_string(r"\xC3\xA9", 0).unwrap(), "é");
        assert!(unescape_string(r"\xC3", 0).is_err());
        assert!(unescape_string(r"\xff", 0).is_err());
    }

    #[test]
    fn calendar_dates() {
        assert!(is_datetime("2024-02-29"));
        assert!(!is_datetime("2023-02-29"));
        assert!(!is_datetime("2023-02-30"));
        assert!(!is_datetime("2024-04-31"));
        assert!(!is_datetime("2100-02-29"));
        assert!(is_datetime("2000-02-29"));
        assert!(is_datetime("2024-01-02T23:59:60"));
        assert!(!is_datetime("2024-01-02T23:59:61"));
        assert!(!is_datetime("2024-01-02T03:04:05+24:00"));
        assert!(is_datetime("2024-01-02t03:04:05z"));
        assert!(is_datetime("2024-01-02T03:04:05.1234567890"));
        assert!(!is_datetime("2024-01-02T03:04"));
    }
}
