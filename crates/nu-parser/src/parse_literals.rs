#![allow(clippy::byte_char_slices)]

use crate::{
    Token, TokenContents,
    lex::lex,
    parse_helpers::{
        SPREAD_OPERATOR_STR, extract_spread_record, garbage, is_variable, trim_quotes,
    },
    parse_pipelines::parse_block,
    type_check::check_range_types,
};
use itertools::Itertools;
use log::trace;
use nu_protocol::{
    DidYouMean, FilesizeUnit, IntoSpanned, ParseError, Span, Spanned, SyntaxShape, Type, Unit,
    VarId, ast::*, casing::Casing, engine::StateWorkingSet,
};
use std::sync::Arc;

use crate::parse_expressions::{
    parse_block_expression, parse_closure_expression, parse_match_block_expression, parse_record,
    parse_table_expression,
};
use crate::parse_signatures::parse_signature;

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
        garbage(working_set, span)
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

    if let Some(token) = token.strip_prefix(prefix)
        && let Some(token) = token.strip_suffix(suffix)
    {
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
                | TokenContents::OutErrGreaterGreaterThan
                | TokenContents::AssignmentOperator => {
                    working_set.error(ParseError::Expected("binary", span));
                    return garbage(working_set, span);
                }
                TokenContents::Comment | TokenContents::Semicolon | TokenContents::Eol => {}
            }
        }

        let required_padding =
            (min_digits_per_byte - binary_value.len() % min_digits_per_byte) % min_digits_per_byte;

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
            Ok(v) => return Expression::new(working_set, Expr::Binary(v), span, Type::Binary),
            Err(help) => {
                working_set.error(ParseError::InvalidBinaryString(span, help.to_string()));
                return garbage(working_set, span);
            }
        }
    }

    working_set.error(ParseError::Expected("binary", span));
    garbage(working_set, span)
}

fn decode_with_base(s: &str, base: u32, digits_per_byte: usize) -> Result<Vec<u8>, &str> {
    s.chars()
        .chunks(digits_per_byte)
        .into_iter()
        .map(|chunk| {
            let str: String = chunk.collect();
            u8::from_str_radix(&str, base).map_err(|_| match base {
                2 => "binary strings may contain only 0 or 1.",
                8 => "octal strings must have a length that is a multiple of three and contain values between 0o000 and 0o377.",
                16 => "hexadecimal strings may contain only the characters 0–9 and A–F.",
                _ => "internal error: radix other than 2, 8, or 16 is not allowed."
            })
        })
        .collect()
}

fn strip_underscores(token: &[u8]) -> String {
    String::from_utf8_lossy(token)
        .chars()
        .filter(|c| *c != '_')
        .collect()
}

pub fn parse_int(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let token = working_set.get_span_contents(span);

    fn extract_int(
        working_set: &mut StateWorkingSet,
        token: &str,
        span: Span,
        radix: u32,
    ) -> Expression {
        // Parse as a u64, then cast to i64, otherwise, for numbers like "0xffffffffffffffef",
        // you'll get `Error parsing hex string: number too large to fit in target type`.
        if let Ok(num) = u64::from_str_radix(token, radix).map(|val| val as i64) {
            Expression::new(working_set, Expr::Int(num), span, Type::Int)
        } else {
            working_set.error(ParseError::InvalidLiteral(
                format!("invalid digits for radix {radix}"),
                "int".into(),
                span,
            ));

            garbage(working_set, span)
        }
    }

    let token = strip_underscores(token);

    if token.is_empty() {
        working_set.error(ParseError::Expected("int", span));
        return garbage(working_set, span);
    }

    if let Some(num) = token.strip_prefix("0b") {
        extract_int(working_set, num, span, 2)
    } else if let Some(num) = token.strip_prefix("0o") {
        extract_int(working_set, num, span, 8)
    } else if let Some(num) = token.strip_prefix("0x") {
        extract_int(working_set, num, span, 16)
    } else if let Ok(num) = token.parse::<i64>() {
        Expression::new(working_set, Expr::Int(num), span, Type::Int)
    } else {
        working_set.error(ParseError::Expected("int", span));
        garbage(working_set, span)
    }
}

pub fn parse_float(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let token = working_set.get_span_contents(span);
    let token = strip_underscores(token);

    if let Ok(x) = token.parse::<f64>() {
        Expression::new(working_set, Expr::Float(x), span, Type::Float)
    } else {
        working_set.error(ParseError::Expected("float", span));

        garbage(working_set, span)
    }
}

pub fn parse_number(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let starting_error_count = working_set.parse_errors.len();

    let result = parse_int(working_set, span);
    if starting_error_count == working_set.parse_errors.len() {
        return result;
    } else if let Some(ParseError::Expected(_, _)) = working_set.parse_errors.last() {
        working_set.parse_errors.truncate(starting_error_count);
    }

    let result = parse_float(working_set, span);

    if starting_error_count == working_set.parse_errors.len() {
        return result;
    }
    working_set.parse_errors.truncate(starting_error_count);

    working_set.error(ParseError::Expected("number", span));
    garbage(working_set, span)
}

pub fn parse_range(working_set: &mut StateWorkingSet, span: Span) -> Option<Expression> {
    trace!("parsing: range");
    let starting_error_count = working_set.parse_errors.len();

    // Range follows the following syntax: [<from>][<next_operator><next>]<range_operator>[<to>]
    //   where <next_operator> is ".."
    //   and  <range_operator> is "..", "..=" or "..<"
    //   and one of the <from> or <to> bounds must be present (just '..' is not allowed since it
    //     looks like parent directory)
    //bugbug range cannot be [..] because that looks like parent directory

    let contents = working_set.get_span_contents(span);

    let Ok(token) = String::from_utf8(contents.into()) else {
        working_set.error(ParseError::NonUtf8(span));
        return None;
    };

    if token.starts_with(SPREAD_OPERATOR_STR) {
        working_set.error(ParseError::Expected(
            "range operator ('..'), got spread ('...')",
            span,
        ));
        return None;
    }

    if !token.contains("..") {
        working_set.error(ParseError::Expected("at least one range bound set", span));
        return None;
    }

    let dotdot_pos: Vec<_> = token
        .match_indices("..")
        .filter_map(|(pos, _)| {
            // paren_depth = count of unclosed parens prior to pos
            let before = &token[..pos];
            let paren_opened = before.chars().filter(|&c| c == '(').count();
            let paren_closed = before.chars().filter(|&c| c == ')').count();
            let paren_depth = paren_opened.checked_sub(paren_closed)?;
            (paren_depth == 0).then_some(pos)
        })
        .collect();

    let (next_op_pos, range_op_pos) = match dotdot_pos.len() {
        1 => (None, dotdot_pos[0]),
        2 => (Some(dotdot_pos[0]), dotdot_pos[1]),
        _ => {
            working_set.error(ParseError::Expected(
                "one range operator ('..' or '..<') and optionally one next operator ('..')",
                span,
            ));
            return None;
        }
    };
    // Avoid calling sub-parsers on unmatched parens, to prevent quadratic time on things like ((((1..2))))
    // No need to call the expensive parse_value on "((((1"
    if dotdot_pos[0] > 0 {
        let (_tokens, err) = lex(
            &contents[..dotdot_pos[0]],
            span.start,
            &[],
            &[b'.', b'?', b'!'],
            true,
        );
        if let Some(_err) = err {
            working_set.error(ParseError::Expected("Valid expression before ..", span));
            return None;
        }
    }

    let (inclusion, range_op_str, range_op_span) = if let Some(pos) = token.find("..<") {
        if pos == range_op_pos {
            let op_str = "..<";
            let op_span = Span::new(
                span.start + range_op_pos,
                span.start + range_op_pos + op_str.len(),
            );
            (RangeInclusion::RightExclusive, "..<", op_span)
        } else {
            working_set.error(ParseError::Expected(
                "inclusive operator preceding second range bound",
                span,
            ));
            return None;
        }
    } else {
        let op_str = if token[range_op_pos..].starts_with("..=") {
            "..="
        } else {
            ".."
        };

        let op_span = Span::new(
            span.start + range_op_pos,
            span.start + range_op_pos + op_str.len(),
        );
        (RangeInclusion::Inclusive, op_str, op_span)
    };

    // Now, based on the operator positions, figure out where the bounds & next are located and
    // parse them
    // TODO: Actually parse the next number in the range
    let from = if token.starts_with("..") {
        // token starts with either next operator, or range operator -- we don't care which one
        None
    } else {
        let from_span = Span::new(span.start, span.start + dotdot_pos[0]);
        Some(crate::parser::parse_value(
            working_set,
            from_span,
            &SyntaxShape::Number,
        ))
    };

    let to = if token.ends_with(range_op_str) {
        None
    } else {
        let to_span = Span::new(range_op_span.end, span.end);
        Some(crate::parser::parse_value(
            working_set,
            to_span,
            &SyntaxShape::Number,
        ))
    };

    trace!("-- from: {from:?} to: {to:?}");

    if let (None, None) = (&from, &to) {
        working_set.error(ParseError::Expected("at least one range bound set", span));
        return None;
    }

    let (next, next_op_span) = if let Some(pos) = next_op_pos {
        let next_op_span = Span::new(span.start + pos, span.start + pos + "..".len());
        let next_span = Span::new(next_op_span.end, range_op_span.start);

        (
            Some(crate::parser::parse_value(
                working_set,
                next_span,
                &SyntaxShape::Number,
            )),
            next_op_span,
        )
    } else {
        (None, span)
    };

    if working_set.parse_errors.len() != starting_error_count {
        return None;
    }

    let operator = RangeOperator {
        inclusion,
        span: range_op_span,
        next_op_span,
    };

    let mut range = Range {
        from,
        next,
        to,
        operator,
    };

    check_range_types(working_set, &mut range);

    Some(Expression::new(
        working_set,
        Expr::Range(Box::new(range)),
        span,
        Type::Range,
    ))
}

pub(crate) fn parse_dollar_expr(
    working_set: &mut StateWorkingSet,
    span: Span,
    shape: &SyntaxShape,
) -> Expression {
    trace!("parsing: dollar expression");
    let contents = working_set.get_span_contents(span);

    if contents.starts_with(b"$\"") || contents.starts_with(b"$'") {
        if matches!(shape, SyntaxShape::GlobPattern) && is_bare_string_interpolation(contents) {
            parse_glob_pattern(working_set, span)
        } else {
            parse_string_interpolation(working_set, span)
        }
    } else if contents.starts_with(b"$.") {
        parse_simple_cell_path(working_set, Span::new(span.start + 2, span.end))
    } else {
        let starting_error_count = working_set.parse_errors.len();

        if let Some(expr) = parse_range(working_set, span) {
            expr
        } else {
            working_set.parse_errors.truncate(starting_error_count);
            parse_full_cell_path(working_set, None, span)
        }
    }
}

pub fn parse_raw_string(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: raw-string, with required delimiters");

    let bytes = working_set.get_span_contents(span);

    let prefix_sharp_cnt = if bytes.starts_with(b"r#") {
        // actually `sharp_cnt` is always `index - 1`
        // but create a variable here to make it clearer.
        let mut sharp_cnt = 1;
        let mut index = 2;
        while index < bytes.len() && bytes[index] == b'#' {
            index += 1;
            sharp_cnt += 1;
        }
        sharp_cnt
    } else {
        working_set.error(ParseError::Expected("r#", span));
        return garbage(working_set, span);
    };
    let expect_postfix_sharp_cnt = prefix_sharp_cnt;
    // check the length of whole raw string.
    // the whole raw string should contains at least
    // 1(r) + prefix_sharp_cnt + 1(') + 1(') + postfix_sharp characters
    if bytes.len() < prefix_sharp_cnt + expect_postfix_sharp_cnt + 3 {
        working_set.error(ParseError::Unclosed("'", span));
        return garbage(working_set, span);
    }

    // check for unbalanced # and single quotes.
    let postfix_bytes = &bytes[bytes.len() - expect_postfix_sharp_cnt..bytes.len()];
    if postfix_bytes.iter().any(|b| *b != b'#') {
        working_set.error(ParseError::Unbalanced("prefix #", "postfix #", span));
        return garbage(working_set, span);
    }
    // check for unblanaced single quotes.
    if bytes[1 + prefix_sharp_cnt] != b'\''
        || bytes[bytes.len() - expect_postfix_sharp_cnt - 1] != b'\''
    {
        working_set.error(ParseError::Unclosed("'", span));
        return garbage(working_set, span);
    }

    let bytes = &bytes[prefix_sharp_cnt + 1 + 1..bytes.len() - 1 - prefix_sharp_cnt];
    if let Ok(token) = String::from_utf8(bytes.into()) {
        Expression::new(working_set, Expr::RawString(token), span, Type::String)
    } else {
        working_set.error(ParseError::Expected("utf8 raw-string", span));
        garbage(working_set, span)
    }
}

pub fn parse_paren_expr(
    working_set: &mut StateWorkingSet,
    span: Span,
    shape: &SyntaxShape,
) -> Expression {
    let starting_error_count = working_set.parse_errors.len();

    if let Some(expr) = parse_range(working_set, span) {
        return expr;
    }

    working_set.parse_errors.truncate(starting_error_count);

    if let SyntaxShape::Signature = shape {
        return parse_signature(working_set, span, false);
    }

    if let SyntaxShape::ExternalSignature = shape {
        return parse_signature(working_set, span, true);
    }

    let fcp_expr = parse_full_cell_path(working_set, None, span);
    let fcp_error_count = working_set.parse_errors.len();
    if fcp_error_count > starting_error_count {
        let malformed_subexpr = working_set.parse_errors[starting_error_count..]
            .first()
            .is_some_and(|e| match e {
                ParseError::Unclosed(right, _) if (*right == ")") => true,
                ParseError::Unbalanced(left, right, _) if *left == "(" && *right == ")" => true,
                _ => false,
            });
        if malformed_subexpr {
            working_set.parse_errors.truncate(starting_error_count);
            if matches!(shape, SyntaxShape::GlobPattern) {
                parse_glob_pattern(working_set, span)
            } else {
                parse_string_interpolation(working_set, span)
            }
        } else {
            fcp_expr
        }
    } else {
        fcp_expr
    }
}

pub fn parse_brace_expr(
    working_set: &mut StateWorkingSet,
    span: Span,
    shape: &SyntaxShape,
) -> Expression {
    // Try to detect what kind of value we're about to parse
    // FIXME: In the future, we should work over the token stream so we only have to do this once
    // before parsing begins

    // FIXME: we're still using the shape because we rely on it to know how to handle syntax where
    // the parse is ambiguous. We'll need to update the parts of the grammar where this is ambiguous
    // and then revisit the parsing.

    if span.end <= (span.start + 1) {
        working_set.error(ParseError::ExpectedWithStringMsg(
            format!("non-block value: {shape}"),
            span,
        ));
        return Expression::garbage(working_set, span);
    }
    let bytes = working_set.get_span_contents(Span::new(span.start + 1, span.end - 1));
    let (tokens, _) = lex(bytes, span.start + 1, &[b'\r', b'\n', b'\t'], &[b':'], true);

    match tokens.as_slice() {
        // If we're empty, that means an empty record or closure
        [] => match shape {
            SyntaxShape::Closure(_) => parse_closure_expression(working_set, shape, span),
            SyntaxShape::Block => parse_block_expression(working_set, span),
            SyntaxShape::MatchBlock => parse_match_block_expression(working_set, span),
            _ => parse_record(working_set, span),
        },
        [
            Token {
                contents: TokenContents::Pipe | TokenContents::PipePipe,
                ..
            },
            ..,
        ] => {
            if let SyntaxShape::Block = shape {
                working_set.error(ParseError::Mismatch("block".into(), "closure".into(), span));
                return Expression::garbage(working_set, span);
            }
            parse_closure_expression(working_set, shape, span)
        }
        [_, third, ..] if working_set.get_span_contents(third.span) == b":" => {
            parse_full_cell_path(working_set, None, span)
        }
        [second, ..] => {
            let second_bytes = working_set.get_span_contents(second.span);
            match shape {
                SyntaxShape::Closure(_) => parse_closure_expression(working_set, shape, span),
                SyntaxShape::Block => parse_block_expression(working_set, span),
                SyntaxShape::MatchBlock => parse_match_block_expression(working_set, span),
                // For edge case of `{}.foo?`, #17896
                _ if second_bytes == b"}" => parse_full_cell_path(working_set, None, span),
                _ if extract_spread_record(second_bytes.into_spanned(second.span)).is_some() => {
                    parse_record(working_set, span)
                }
                SyntaxShape::Any => parse_closure_expression(working_set, shape, span),
                _ => {
                    working_set.error(ParseError::ExpectedWithStringMsg(
                        format!("non-block value: {shape}"),
                        span,
                    ));

                    Expression::garbage(working_set, span)
                }
            }
        }
    }
}

pub fn parse_string_interpolation(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    #[derive(PartialEq, Eq, Debug)]
    enum InterpolationMode {
        String,
        Expression,
    }

    let contents = working_set.get_span_contents(span);

    let mut double_quote = false;

    let (start, end) = if contents.starts_with(b"$\"") {
        double_quote = true;

        if let Err(err) = check_string_no_trailing_tokens(contents, span, 1, b'\"') {
            working_set.error(err);
            return garbage(working_set, span);
        }

        let end = if contents.ends_with(b"\"") && contents.len() > 2 {
            span.end - 1
        } else {
            span.end
        };
        (span.start + 2, end)
    } else if contents.starts_with(b"$'") {
        if let Err(err) = check_string_no_trailing_tokens(contents, span, 1, b'\'') {
            working_set.error(err);
            return garbage(working_set, span);
        }

        let end = if contents.ends_with(b"'") && contents.len() > 2 {
            span.end - 1
        } else {
            span.end
        };
        (span.start + 2, end)
    } else {
        (span.start, span.end)
    };

    let inner_span = Span::new(start, end);
    let contents = working_set.get_span_contents(inner_span).to_vec();

    let mut output = vec![];
    let mut mode = InterpolationMode::String;
    let mut token_start = start;

    #[repr(u8)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Delimiter {
        SingleQuote = b'\'',
        DoubleQuote = b'"',
        Backtick = b'`',
        ParenLeft = b'(',
        ParenRight = b')',
    }

    impl Delimiter {
        const fn from_u8(b: u8) -> Option<Self> {
            Some(match b {
                b'\'' => Self::SingleQuote,
                b'"' => Self::DoubleQuote,
                b'`' => Self::Backtick,
                b'(' => Self::ParenLeft,
                b')' => Self::ParenRight,
                _ => return None,
            })
        }
        const fn is_paren(self) -> bool {
            matches!(self, Self::ParenLeft | Self::ParenRight)
        }
        const fn pair(self) -> Self {
            match self {
                Self::ParenLeft => Self::ParenRight,
                Self::ParenRight => Self::ParenLeft,
                _ => self,
            }
        }
    }
    let mut delimiter_stack: Vec<Delimiter> = vec![];

    let mut consecutive_backslashes: usize = 0;

    let mut b = start;

    while b != end {
        let current_byte = contents[b - start];

        if mode == InterpolationMode::String {
            let preceding_consecutive_backslashes = consecutive_backslashes;

            let is_backslash = current_byte == b'\\';
            consecutive_backslashes = if is_backslash {
                preceding_consecutive_backslashes + 1
            } else {
                0
            };

            if current_byte == b'('
                && (!double_quote || preceding_consecutive_backslashes.is_multiple_of(2))
            {
                mode = InterpolationMode::Expression;
                if token_start < b {
                    let span = Span::new(token_start, b);
                    let str_contents = working_set.get_span_contents(span);

                    let (str_contents, err) = if double_quote {
                        unescape_string(str_contents, span)
                    } else {
                        (str_contents.to_vec(), None)
                    };
                    if let Some(err) = err {
                        working_set.error(err);
                    }

                    output.push(Expression::new(
                        working_set,
                        Expr::String(String::from_utf8_lossy(&str_contents).to_string()),
                        span,
                        Type::String,
                    ));
                    token_start = b;
                }
            }
        }

        if mode == InterpolationMode::Expression {
            let byte = Delimiter::from_u8(current_byte);
            match (delimiter_stack.last().copied(), byte) {
                (Some(d), Some(byte)) if !d.is_paren() => {
                    if byte == d {
                        delimiter_stack.pop();
                    }
                }
                (_, Some(byte)) if byte != Delimiter::ParenRight => {
                    delimiter_stack.push(byte.pair())
                }
                (d, Some(Delimiter::ParenRight)) => {
                    if let Some(Delimiter::ParenRight) = d {
                        delimiter_stack.pop();
                    }
                    if delimiter_stack.is_empty() {
                        mode = InterpolationMode::String;

                        if token_start < b {
                            let span = Span::new(token_start, b + 1);

                            let expr = parse_full_cell_path(working_set, None, span);
                            output.push(expr);
                        }

                        token_start = b + 1;
                        continue;
                    }
                }
                _ => (),
            }
        }
        b += 1;
    }

    match mode {
        InterpolationMode::String => {
            if token_start < end {
                let span = Span::new(token_start, end);
                let str_contents = working_set.get_span_contents(span);

                let (str_contents, err) = if double_quote {
                    unescape_string(str_contents, span)
                } else {
                    (str_contents.to_vec(), None)
                };
                if let Some(err) = err {
                    working_set.error(err);
                }

                output.push(Expression::new(
                    working_set,
                    Expr::String(String::from_utf8_lossy(&str_contents).to_string()),
                    span,
                    Type::String,
                ));
            }
        }
        InterpolationMode::Expression => {
            if token_start < end {
                let span = Span::new(token_start, end);
                let expr = parse_full_cell_path(working_set, None, span);
                output.push(expr);
            }
        }
    }

    Expression::new(
        working_set,
        Expr::StringInterpolation(output),
        span,
        Type::String,
    )
}

pub fn parse_variable_expr(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let contents = working_set.get_span_contents(span);

    if contents == b"$nu" {
        return Expression::new(
            working_set,
            Expr::Var(nu_protocol::NU_VARIABLE_ID),
            span,
            Type::Any,
        );
    } else if contents == b"$in" {
        return Expression::new(
            working_set,
            Expr::Var(nu_protocol::IN_VARIABLE_ID),
            span,
            Type::Any,
        );
    } else if contents == b"$env" {
        return Expression::new(
            working_set,
            Expr::Var(nu_protocol::ENV_VARIABLE_ID),
            span,
            Type::Any,
        );
    }

    let name = if contents.starts_with(b"$") {
        String::from_utf8_lossy(&contents[1..]).to_string()
    } else {
        String::from_utf8_lossy(contents).to_string()
    };

    let bytes = working_set.get_span_contents(span);
    let suggestion = || {
        DidYouMean::new(
            &working_set.list_variables(),
            working_set.get_span_contents(span),
        )
    };
    if !is_variable(bytes) {
        working_set.error(ParseError::ExpectedWithDidYouMean(
            "valid variable name",
            suggestion(),
            span,
        ));
        garbage(working_set, span)
    } else if let Some(id) = working_set.find_variable(bytes) {
        Expression::new(
            working_set,
            Expr::Var(id),
            span,
            working_set.get_variable(id).ty.clone(),
        )
    } else if working_set.get_env_var(&name).is_some() {
        working_set.error(ParseError::EnvVarNotVar(name, span));
        garbage(working_set, span)
    } else {
        working_set.error(ParseError::VariableNotFound(suggestion(), span));
        garbage(working_set, span)
    }
}

pub fn parse_cell_path(
    working_set: &mut StateWorkingSet,
    tokens: impl Iterator<Item = Token>,
    expect_dot: bool,
) -> Vec<PathMember> {
    enum TokenType {
        Dot,              // .
        DotOrSign,        // . or ? or !
        DotOrExclamation, // . or !
        DotOrQuestion,    // . or ?
        PathMember,       // an int or string, like `1` or `foo`
    }

    enum ModifyMember {
        No,
        Optional,
        Insensitive,
    }

    impl TokenType {
        fn expect(&mut self, byte: u8) -> Result<ModifyMember, &'static str> {
            match (&*self, byte) {
                (Self::PathMember, _) => {
                    *self = Self::DotOrSign;
                    Ok(ModifyMember::No)
                }
                (
                    Self::Dot | Self::DotOrSign | Self::DotOrExclamation | Self::DotOrQuestion,
                    b'.',
                ) => {
                    *self = Self::PathMember;
                    Ok(ModifyMember::No)
                }
                (Self::DotOrSign, b'!') => {
                    *self = Self::DotOrQuestion;
                    Ok(ModifyMember::Insensitive)
                }
                (Self::DotOrSign, b'?') => {
                    *self = Self::DotOrExclamation;
                    Ok(ModifyMember::Optional)
                }
                (Self::DotOrSign, _) => Err(". or ! or ?"),
                (Self::DotOrExclamation, b'!') => {
                    *self = Self::Dot;
                    Ok(ModifyMember::Insensitive)
                }
                (Self::DotOrExclamation, _) => Err(". or !"),
                (Self::DotOrQuestion, b'?') => {
                    *self = Self::Dot;
                    Ok(ModifyMember::Optional)
                }
                (Self::DotOrQuestion, _) => Err(". or ?"),
                (Self::Dot, _) => Err("."),
            }
        }
    }

    // Parsing a cell path is essentially a state machine, and this is the state
    let mut expected_token = if expect_dot {
        TokenType::Dot
    } else {
        TokenType::PathMember
    };

    let mut tail = vec![];

    for path_element in tokens {
        let bytes = working_set.get_span_contents(path_element.span);

        // both parse_int and parse_string require their source to be non-empty
        // all cases where `bytes` is empty is an error
        let Some((&first, rest)) = bytes.split_first() else {
            working_set.error(ParseError::Expected("string", path_element.span));
            return tail;
        };
        let single_char = rest.is_empty();

        if let TokenType::PathMember = expected_token {
            let starting_error_count = working_set.parse_errors.len();

            let expr = parse_int(working_set, path_element.span);
            working_set.parse_errors.truncate(starting_error_count);

            match expr {
                Expression {
                    expr: Expr::Int(val),
                    span,
                    ..
                } => {
                    if val < 0 {
                        working_set.error(ParseError::InvalidLiteral(
                            "negative index is not supported".into(),
                            "cell path".into(),
                            span,
                        ));
                        return tail;
                    }
                    tail.push(PathMember::Int {
                        val: val as usize,
                        span,
                        optional: false,
                    })
                }
                _ => {
                    let result = parse_string(working_set, path_element.span);
                    match result {
                        Expression {
                            expr: Expr::String(string),
                            span,
                            ..
                        } => {
                            tail.push(PathMember::String {
                                val: string,
                                span,
                                optional: false,
                                casing: Casing::Sensitive,
                            });
                        }
                        _ => {
                            working_set.error(ParseError::Expected("string", path_element.span));
                            return tail;
                        }
                    }
                }
            }
            expected_token = TokenType::DotOrSign;
        } else {
            match expected_token.expect(if single_char { first } else { b' ' }) {
                Ok(modify) => {
                    if let Some(last) = tail.last_mut() {
                        match modify {
                            ModifyMember::No => {}
                            ModifyMember::Optional => last.make_optional(),
                            ModifyMember::Insensitive => last.make_insensitive(),
                        }
                    };
                }
                Err(expected) => {
                    working_set.error(ParseError::Expected(expected, path_element.span));
                    return tail;
                }
            }
        }
    }

    tail
}

pub fn parse_simple_cell_path(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let source = working_set.get_span_contents(span);

    let (tokens, err) = lex(
        source,
        span.start,
        &[b'\n', b'\r'],
        &[b'.', b'?', b'!'],
        true,
    );
    if let Some(err) = err {
        working_set.error(err)
    }

    let tokens = tokens.into_iter().peekable();

    let cell_path = parse_cell_path(working_set, tokens, false);

    Expression::new(
        working_set,
        Expr::CellPath(CellPath { members: cell_path }),
        span,
        Type::CellPath,
    )
}

pub fn parse_full_cell_path(
    working_set: &mut StateWorkingSet,
    implicit_head: Option<VarId>,
    span: Span,
) -> Expression {
    trace!("parsing: full cell path");
    let full_cell_span = span;
    let source = working_set.get_span_contents(span);

    let (tokens, err) = lex(
        source,
        span.start,
        &[b'\n', b'\r'],
        &[b'.', b'?', b'!'],
        true,
    );
    if let Some(err) = err {
        working_set.error(err)
    }

    let mut tokens = tokens.into_iter().peekable();
    if let Some(head) = tokens.peek() {
        let bytes = working_set.get_span_contents(head.span);
        let (head, expect_dot) = if bytes.starts_with(b"(") {
            trace!("parsing: paren-head of full cell path");

            let head_span = head.span;
            let mut start = head.span.start;
            let mut end = head.span.end;
            let mut is_closed = true;

            if bytes.starts_with(b"(") {
                start += 1;
            }
            if bytes.ends_with(b")") {
                end -= 1;
            } else {
                working_set.error(ParseError::Unclosed(")", Span::new(end, end)));
                is_closed = false;
            }

            let span = Span::new(start, end);

            let source = working_set.get_span_contents(span);

            let (output, err) = lex(source, span.start, &[b'\n', b'\r'], &[], true);
            if let Some(err) = err {
                working_set.error(err)
            }

            // Creating a Type scope to parse the new block. This will keep track of
            // the previous input type found in that block
            let output = parse_block(working_set, &output, span, is_closed, true);

            let ty = output.output_type();

            let block_id = working_set.add_block(Arc::new(output));
            tokens.next();

            (
                Expression::new(working_set, Expr::Subexpression(block_id), head_span, ty),
                true,
            )
        } else if bytes.starts_with(b"[") {
            trace!("parsing: table head of full cell path");

            let output = parse_table_expression(working_set, head.span, &SyntaxShape::Any);

            tokens.next();

            (output, true)
        } else if bytes.starts_with(b"{") {
            trace!("parsing: record head of full cell path");
            let output = parse_record(working_set, head.span);

            tokens.next();

            (output, true)
        } else if bytes.starts_with(b"$") {
            trace!("parsing: $variable head of full cell path");

            let out = parse_variable_expr(working_set, head.span);

            tokens.next();

            (out, true)
        } else if let Some(var_id) = implicit_head {
            trace!("parsing: implicit head of full cell path");
            (
                Expression::new(working_set, Expr::Var(var_id), head.span, Type::Any),
                false,
            )
        } else {
            working_set.error(ParseError::Mismatch(
                "variable or subexpression".into(),
                String::from_utf8_lossy(bytes).to_string(),
                span,
            ));
            return garbage(working_set, span);
        };

        let tail = parse_cell_path(working_set, tokens, expect_dot);
        let ty = if !tail.is_empty() {
            if nu_experimental::CELL_PATH_TYPES.get() {
                head.ty
                    .follow_cell_path(&tail)
                    .map(|ty| ty.into_owned())
                    .unwrap_or(Type::Any)
            } else {
                Type::Any
            }
        } else {
            head.ty.clone()
        };

        Expression::new(
            working_set,
            Expr::FullCellPath(Box::new(FullCellPath { head, tail })),
            full_cell_span,
            ty,
        )
    } else {
        garbage(working_set, span)
    }
}

enum PathLikeKind {
    Directory,
    Filepath,
    Glob,
}

impl PathLikeKind {
    /// Returns the name used for trace logging during parsing.
    fn trace_name(&self) -> &'static str {
        match self {
            PathLikeKind::Directory => "directory",
            PathLikeKind::Filepath => "filepath",
            PathLikeKind::Glob => "glob pattern",
        }
    }

    /// Returns the error message displayed when parsing fails.
    fn error_msg(&self) -> &'static str {
        match self {
            PathLikeKind::Directory => "directory",
            PathLikeKind::Filepath => "filepath",
            PathLikeKind::Glob => "glob pattern string",
        }
    }

    /// Constructs the appropriate `Expr` and its corresponding `Type` for a simple (non-interpolated) path.
    fn to_expr(&self, token: String, quoted: bool) -> (Expr, Type) {
        match self {
            PathLikeKind::Directory => (Expr::Directory(token, quoted), Type::String),
            PathLikeKind::Filepath => (Expr::Filepath(token, quoted), Type::String),
            PathLikeKind::Glob => (Expr::GlobPattern(token, quoted), Type::Glob),
        }
    }

    /// Constructs the appropriate interpolation `Expr` for a path containing subexpressions.
    fn to_interpolation_expr(&self, exprs: Vec<Expression>, quoted: bool) -> Expr {
        match self {
            PathLikeKind::Directory | PathLikeKind::Filepath => Expr::StringInterpolation(exprs),
            PathLikeKind::Glob => Expr::GlobInterpolation(exprs, quoted),
        }
    }
}

/// Common helper for parsing path-like expressions (filepath, directory, glob pattern).
///
/// This function consolidates the repetitive logic for parsing path types, including:
/// - Bare word interpolation detection
/// - Escape sequence processing
/// - Quote state tracking
/// - Error handling
///
/// # Arguments
///
/// * `working_set` - The current parser state
/// * `span` - The source span of the expression
/// * `kind` - The kind of path-like expression to parse
fn parse_path_like(
    working_set: &mut StateWorkingSet,
    span: Span,
    kind: PathLikeKind,
) -> Expression {
    let bytes = working_set.get_span_contents(span);
    let quoted = is_quoted(bytes);
    trace!("parsing: {}", kind.trace_name());

    // Check for bare word interpolation
    if is_bare_string_interpolation(bytes) {
        let interpolation_expr = parse_string_interpolation(working_set, span);

        // Convert StringInterpolation to the appropriate interpolation type
        if let Expr::StringInterpolation(exprs) = interpolation_expr.expr {
            return Expression::new(
                working_set,
                kind.to_interpolation_expr(exprs, quoted),
                span,
                interpolation_expr.ty.clone(),
            );
        }

        return interpolation_expr;
    }

    let (token, err) = unescape_unquote_string(bytes, span);
    let is_quoted_internal = is_quoted(bytes);

    if err.is_none() {
        trace!("-- found {token}");

        let (expr, ty) = kind.to_expr(token, is_quoted_internal);

        Expression::new(working_set, expr, span, ty)
    } else {
        working_set.error(ParseError::Expected(kind.error_msg(), span));

        garbage(working_set, span)
    }
}

fn is_bare_string_interpolation(bytes: &[u8]) -> bool {
    match bytes {
        [] => false,
        [b'\'' | b'"' | b'`', ..] => false,
        _ => bytes.contains(&b'('),
    }
}

pub fn parse_directory(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    parse_path_like(working_set, span, PathLikeKind::Directory)
}

pub fn parse_filepath(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    parse_path_like(working_set, span, PathLikeKind::Filepath)
}

pub fn parse_datetime(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: datetime");

    let bytes = working_set.get_span_contents(span);

    if bytes.len() < 6
        || !bytes[0].is_ascii_digit()
        || !bytes[1].is_ascii_digit()
        || !bytes[2].is_ascii_digit()
        || !bytes[3].is_ascii_digit()
        || bytes[4] != b'-'
    {
        working_set.error(ParseError::Expected("datetime", span));
        return garbage(working_set, span);
    }

    let token = String::from_utf8_lossy(bytes).to_string();

    if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(&token) {
        return Expression::new(working_set, Expr::DateTime(datetime), span, Type::Date);
    }

    // Just the date
    let just_date = token.clone() + "T00:00:00+00:00";
    if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(&just_date) {
        return Expression::new(working_set, Expr::DateTime(datetime), span, Type::Date);
    }

    // Date and time, assume UTC
    let datetime = token + "+00:00";
    if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(&datetime) {
        return Expression::new(working_set, Expr::DateTime(datetime), span, Type::Date);
    }

    working_set.error(ParseError::Expected("datetime", span));

    garbage(working_set, span)
}

/// Parse a duration type, eg '10day'
pub fn parse_duration(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: duration");

    let bytes = working_set.get_span_contents(span);

    match parse_unit_value(bytes, span, DURATION_UNIT_GROUPS, Type::Duration, |x| x) {
        Some(Ok(expr)) => {
            let span_id = working_set.add_span(span);
            expr.with_span_id(span_id)
        }
        Some(Err(mk_err_for)) => {
            working_set.error(mk_err_for("duration"));
            garbage(working_set, span)
        }
        None => {
            working_set.error(ParseError::Expected("duration with valid units", span));
            garbage(working_set, span)
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
        return garbage(working_set, span);
    }

    match parse_unit_value(bytes, span, FILESIZE_UNIT_GROUPS, Type::Filesize, |x| {
        x.to_ascii_uppercase()
    }) {
        Some(Ok(expr)) => {
            let span_id = working_set.add_span(span);
            expr.with_span_id(span_id)
        }
        Some(Err(mk_err_for)) => {
            working_set.error(mk_err_for("filesize"));
            garbage(working_set, span)
        }
        None => {
            working_set.error(ParseError::Expected("filesize with valid units", span));
            garbage(working_set, span)
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
    transform: fn(String) -> String,
) -> Option<ParseUnitResult<'res>> {
    if bytes.len() < 2
        || !(bytes[0].is_ascii_digit()
            || (bytes[0] == b'.' && bytes[1].is_ascii_digit())
            || (bytes[0] == b'-' && bytes[1].is_ascii_digit()))
    {
        return None;
    }

    // Bail if not UTF-8
    let value = transform(str::from_utf8(bytes).ok()?.into());

    if let Some((unit, name, convert)) = unit_groups.iter().find(|x| value.ends_with(x.1)) {
        let lhs_len = value.len() - name.len();
        let lhs = strip_underscores(&value.as_bytes()[..lhs_len]);
        let lhs_span = Span::new(span.start, span.start + lhs_len);
        let unit_span = Span::new(span.start + lhs_len, span.end);
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

        let mut unit = match convert {
            Some(convert_to) => convert_to.0,
            None => *unit,
        };

        let num_float = match convert {
            Some(convert_to) => {
                (number_part * convert_to.1 as f64) + (decimal_part * convert_to.1 as f64)
            }
            None => number_part,
        };

        // Convert all durations to nanoseconds, and filesizes to bytes,
        // to minimize loss of precision
        let factor = match ty {
            Type::Filesize => unit_to_byte_factor(&unit),
            Type::Duration => unit_to_ns_factor(&unit),
            _ => None,
        };

        let num = match factor {
            Some(factor) => {
                let num_base = num_float * factor;
                if i64::MIN as f64 <= num_base && num_base <= i64::MAX as f64 {
                    unit = if ty == Type::Filesize {
                        Unit::Filesize(FilesizeUnit::B)
                    } else {
                        Unit::Nanosecond
                    };
                    num_base as i64
                } else {
                    // not safe to convert, because of the overflow
                    num_float as i64
                }
            }
            None => num_float as i64,
        };

        trace!("-- found {num} {unit:?}");
        let value = ValueWithUnit {
            expr: Expression::new_unknown(Expr::Int(num), lhs_span, Type::Number),
            unit: Spanned {
                item: unit,
                span: unit_span,
            },
        };
        let expr = Expression::new_unknown(Expr::ValueWithUnit(Box::new(value)), span, ty);

        Some(Ok(expr))
    } else {
        None
    }
}

pub const FILESIZE_UNIT_GROUPS: &[UnitGroup] = &[
    (
        Unit::Filesize(FilesizeUnit::KB),
        "KB",
        Some((Unit::Filesize(FilesizeUnit::B), 1000)),
    ),
    (
        Unit::Filesize(FilesizeUnit::MB),
        "MB",
        Some((Unit::Filesize(FilesizeUnit::KB), 1000)),
    ),
    (
        Unit::Filesize(FilesizeUnit::GB),
        "GB",
        Some((Unit::Filesize(FilesizeUnit::MB), 1000)),
    ),
    (
        Unit::Filesize(FilesizeUnit::TB),
        "TB",
        Some((Unit::Filesize(FilesizeUnit::GB), 1000)),
    ),
    (
        Unit::Filesize(FilesizeUnit::PB),
        "PB",
        Some((Unit::Filesize(FilesizeUnit::TB), 1000)),
    ),
    (
        Unit::Filesize(FilesizeUnit::EB),
        "EB",
        Some((Unit::Filesize(FilesizeUnit::PB), 1000)),
    ),
    (
        Unit::Filesize(FilesizeUnit::KiB),
        "KIB",
        Some((Unit::Filesize(FilesizeUnit::B), 1024)),
    ),
    (
        Unit::Filesize(FilesizeUnit::MiB),
        "MIB",
        Some((Unit::Filesize(FilesizeUnit::KiB), 1024)),
    ),
    (
        Unit::Filesize(FilesizeUnit::GiB),
        "GIB",
        Some((Unit::Filesize(FilesizeUnit::MiB), 1024)),
    ),
    (
        Unit::Filesize(FilesizeUnit::TiB),
        "TIB",
        Some((Unit::Filesize(FilesizeUnit::GiB), 1024)),
    ),
    (
        Unit::Filesize(FilesizeUnit::PiB),
        "PIB",
        Some((Unit::Filesize(FilesizeUnit::TiB), 1024)),
    ),
    (
        Unit::Filesize(FilesizeUnit::EiB),
        "EIB",
        Some((Unit::Filesize(FilesizeUnit::PiB), 1024)),
    ),
    (Unit::Filesize(FilesizeUnit::B), "B", None),
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

fn unit_to_ns_factor(unit: &Unit) -> Option<f64> {
    match unit {
        Unit::Nanosecond => Some(1.0),
        Unit::Microsecond => Some(1_000.0),
        Unit::Millisecond => Some(1_000_000.0),
        Unit::Second => Some(1_000_000_000.0),
        Unit::Minute => Some(60.0 * 1_000_000_000.0),
        Unit::Hour => Some(60.0 * 60.0 * 1_000_000_000.0),
        Unit::Day => Some(24.0 * 60.0 * 60.0 * 1_000_000_000.0),
        Unit::Week => Some(7.0 * 24.0 * 60.0 * 60.0 * 1_000_000_000.0),
        _ => None,
    }
}

fn unit_to_byte_factor(unit: &Unit) -> Option<f64> {
    match unit {
        Unit::Filesize(FilesizeUnit::B) => Some(1.0),
        Unit::Filesize(FilesizeUnit::KB) => Some(1_000.0),
        Unit::Filesize(FilesizeUnit::MB) => Some(1_000_000.0),
        Unit::Filesize(FilesizeUnit::GB) => Some(1_000_000_000.0),
        Unit::Filesize(FilesizeUnit::TB) => Some(1_000_000_000_000.0),
        Unit::Filesize(FilesizeUnit::PB) => Some(1_000_000_000_000_000.0),
        Unit::Filesize(FilesizeUnit::EB) => Some(1_000_000_000_000_000_000.0),
        Unit::Filesize(FilesizeUnit::KiB) => Some(1024.0),
        Unit::Filesize(FilesizeUnit::MiB) => Some(1024.0 * 1024.0),
        Unit::Filesize(FilesizeUnit::GiB) => Some(1024.0 * 1024.0 * 1024.0),
        Unit::Filesize(FilesizeUnit::TiB) => Some(1024.0 * 1024.0 * 1024.0 * 1024.0),
        Unit::Filesize(FilesizeUnit::PiB) => Some(1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0),
        Unit::Filesize(FilesizeUnit::EiB) => {
            Some(1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0)
        }
        _ => None,
    }
}

// Borrowed from libm at https://github.com/rust-lang/libm/blob/master/src/math/modf.rs
fn modf(x: f64) -> (f64, f64) {
    let rv2: f64;
    let mut u = x.to_bits();
    let e = (((u >> 52) & 0x7ff) as i32) - 0x3ff;

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

pub fn parse_glob_pattern(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    parse_path_like(working_set, span, PathLikeKind::Glob)
}

fn parse_hex_escape(bytes: &[u8], start_idx: usize, span: Span) -> Result<(u8, usize), ParseError> {
    let hex_digits = bytes.get(start_idx + 1..start_idx + 3).ok_or_else(|| {
        ParseError::InvalidLiteral(
            "incomplete hex escape '\\xHH', expected 2 hex digits".into(),
            "string".into(),
            Span::new(span.start + start_idx, span.end),
        )
    })?;
    if !hex_digits.iter().all(u8::is_ascii_hexdigit) {
        return Err(ParseError::InvalidLiteral(
            "invalid hex escape '\\xHH', expected exactly 2 hex digits".into(),
            "string".into(),
            Span::new(span.start + start_idx, span.end),
        ));
    }
    str::from_utf8(hex_digits)
        .ok()
        .and_then(|s| u8::from_str_radix(s, 0x10).ok())
        .map(|byte_val| (byte_val, start_idx + 3))
        .ok_or_else(|| {
            ParseError::InvalidLiteral(
                "invalid hex escape '\\xHH'".into(),
                "string".into(),
                Span::new(span.start + start_idx, span.end),
            )
        })
}

fn parse_unicode_escape(
    bytes: &[u8],
    start_idx: usize,
    span: Span,
) -> Result<(char, usize), ParseError> {
    let mut slice = &bytes[(start_idx + 1)..];
    let mut current_idx = start_idx + 1;

    // NOTE: this is a more defensive approach meant to avoid reading too much, but requires
    //       changing error messages
    // read no more than 8 bytes "{xxxxxx}"
    // slice = &slice[..(8.min(slice.len()))];

    slice = slice.strip_prefix(b"{").ok_or_else(|| {
        ParseError::InvalidLiteral(
            "invalid unicode escape '\\u{...}', must be 1-6 hex digits, max codepoint 0x10FFFF"
                .into(),
            "string".into(),
            Span::new(span.start + start_idx, span.end),
        )
    })?;
    current_idx += 1;

    let end = slice.iter().position(|b| *b == b'}').ok_or_else(|| {
        ParseError::InvalidLiteral(
            "incomplete unicode escape '\\u{...}', missing closing '}'".into(),
            "string".into(),
            Span::new(span.start + start_idx, span.end),
        )
    })?;
    let digits = &slice[..end];
    current_idx += end; // the digits
    current_idx += 1; // closing brace
    let current_idx = current_idx;

    let ch = Some(digits)
        .filter(|b| (1..=6).contains(&b.len()))
        .and_then(|b| str::from_utf8(b).ok())
        .and_then(|s| u32::from_str_radix(s, 0x10).ok())
        .and_then(char::from_u32)
        .ok_or_else(|| {
            ParseError::InvalidLiteral(
                "invalid unicode escape '\\u{...}', must be 1-6 hex digits, max codepoint 0x10FFFF"
                    .into(),
                "string".into(),
                Span::new(span.start + start_idx, span.end),
            )
        })?;

    Ok((ch, current_idx))
}

pub fn unescape_string(bytes: &[u8], span: Span) -> (Vec<u8>, Option<ParseError>) {
    let mut output = Vec::new();
    let mut error = None;

    let mut idx = 0;

    if !bytes.contains(&b'\\') {
        return (bytes.to_vec(), None);
    }

    'us_loop: while idx < bytes.len() {
        if bytes[idx] == b'\\' {
            // We're in an escape
            idx += 1;

            match bytes.get(idx) {
                Some(b'"') => {
                    output.push(b'"');
                    idx += 1;
                }
                Some(b'\'') => {
                    output.push(b'\'');
                    idx += 1;
                }
                Some(b'\\') => {
                    output.push(b'\\');
                    idx += 1;
                }
                Some(b'/') => {
                    output.push(b'/');
                    idx += 1;
                }
                Some(b'(') => {
                    output.push(b'(');
                    idx += 1;
                }
                Some(b')') => {
                    output.push(b')');
                    idx += 1;
                }
                Some(b'{') => {
                    output.push(b'{');
                    idx += 1;
                }
                Some(b'}') => {
                    output.push(b'}');
                    idx += 1;
                }
                Some(b'$') => {
                    output.push(b'$');
                    idx += 1;
                }
                Some(b'^') => {
                    output.push(b'^');
                    idx += 1;
                }
                Some(b'#') => {
                    output.push(b'#');
                    idx += 1;
                }
                Some(b'|') => {
                    output.push(b'|');
                    idx += 1;
                }
                Some(b'~') => {
                    output.push(b'~');
                    idx += 1;
                }
                Some(b'a') => {
                    output.push(0x7);
                    idx += 1;
                }
                Some(b'b') => {
                    output.push(0x8);
                    idx += 1;
                }
                Some(b'e') => {
                    output.push(0x1b);
                    idx += 1;
                }
                Some(b'f') => {
                    output.push(0xc);
                    idx += 1;
                }
                Some(b'n') => {
                    output.push(b'\n');
                    idx += 1;
                }
                Some(b'r') => {
                    output.push(b'\r');
                    idx += 1;
                }
                Some(b't') => {
                    output.push(b'\t');
                    idx += 1;
                }
                Some(b'0') => {
                    output.push(b'\0');
                    idx += 1;
                }
                Some(b'x') => {
                    // Hex escape: \xHH (exactly 2 hex digits)
                    match parse_hex_escape(bytes, idx, span) {
                        Ok((byte_val, new_idx)) => {
                            output.push(byte_val);
                            idx = new_idx;
                        }
                        Err(err) => {
                            error = error.or(Some(err));
                            break 'us_loop;
                        }
                    }
                }
                Some(b'u') => {
                    // Unicode escape: \u{XXXXXX} (1-6 hex digits, max 0x10FFFF)
                    match parse_unicode_escape(bytes, idx, span) {
                        Ok((ch, new_idx)) => {
                            let mut ch_buf = [0u8; 4];
                            output.extend(ch.encode_utf8(&mut ch_buf).as_bytes());
                            idx = new_idx;
                        }
                        Err(err) => {
                            error = error.or(Some(err));
                            break 'us_loop;
                        }
                    }
                }

                Some(other) => {
                    error = error.or(Some(ParseError::InvalidLiteral(
                        format!("unrecognized escape sequence '\\{}'", *other as char),
                        "string".into(),
                        Span::new(span.start + idx, span.end),
                    )));
                    break 'us_loop;
                }
                None => {
                    error = error.or(Some(ParseError::InvalidLiteral(
                        "incomplete escape sequence after '\\'".into(),
                        "string".into(),
                        Span::new(span.end.saturating_sub(1), span.end),
                    )));
                    break 'us_loop;
                }
            }
        } else {
            output.push(bytes[idx]);
            idx += 1;
        }
    }

    (output, error)
}

pub fn unescape_unquote_string(bytes: &[u8], span: Span) -> (String, Option<ParseError>) {
    if bytes.starts_with(b"\"") {
        // Needs unescaping
        let bytes = trim_quotes(bytes);

        let (bytes, err) = unescape_string(bytes, span);

        if let Ok(token) = String::from_utf8(bytes) {
            (token, err)
        } else {
            (String::new(), Some(ParseError::Expected("string", span)))
        }
    } else {
        let bytes = trim_quotes(bytes);

        if let Ok(token) = String::from_utf8(bytes.into()) {
            (token, None)
        } else {
            (String::new(), Some(ParseError::Expected("string", span)))
        }
    }
}

fn check_string_no_trailing_tokens(
    bytes: &[u8],
    span: Span,
    opening_quote_pos: usize,
    quote: u8,
) -> Result<(), ParseError> {
    let pos = bytes
        .iter()
        .rposition(|ch| *ch == quote)
        .expect("string begins with quote");
    if pos == bytes.len() - 1 {
        Ok(())
    } else if pos == opening_quote_pos {
        // this may look like an error, but it's not:
        // some code, like completions, requires allowing
        // unterminated strings at this stage.
        Ok(())
    } else {
        let span = Span::new(span.start + pos + 1, span.end);
        Err(ParseError::ExtraTokensAfterClosingDelimiter(span))
    }
}

pub fn parse_string(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: string");

    let bytes = working_set.get_span_contents(span);

    if bytes.is_empty() {
        working_set.error(ParseError::Expected("String", span));
        return Expression::garbage(working_set, span);
    }

    // Check for bare word interpolation
    if is_bare_string_interpolation(bytes) {
        return parse_string_interpolation(working_set, span);
    }

    // Check for unbalanced quotes:
    for quote in [b'\"', b'\''] {
        if bytes[0] == quote
            && let Err(err) = check_string_no_trailing_tokens(bytes, span, 0, quote)
        {
            working_set.error(err);
            return garbage(working_set, span);
        }
    }

    let (s, err) = unescape_unquote_string(bytes, span);
    if let Some(err) = err {
        working_set.error(err);
    }

    Expression::new(working_set, Expr::String(s), span, Type::String)
}

fn is_quoted(bytes: &[u8]) -> bool {
    matches!(bytes, [b'\'', .., b'\''] | [b'"', .., b'"'])
}

pub fn parse_string_strict(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: string, with required delimiters");

    let bytes = working_set.get_span_contents(span);

    // Check for unbalanced quotes:
    {
        let bytes = if bytes.starts_with(b"$") {
            &bytes[1..]
        } else {
            bytes
        };
        if bytes.starts_with(b"\"") && (bytes.len() == 1 || !bytes.ends_with(b"\"")) {
            working_set.error(ParseError::Unclosed("\"", span));
            return garbage(working_set, span);
        }
        if bytes.starts_with(b"\'") && (bytes.len() == 1 || !bytes.ends_with(b"\'")) {
            working_set.error(ParseError::Unclosed("\'", span));
            return garbage(working_set, span);
        }
        if bytes.starts_with(b"r#") && (bytes.len() == 1 || !bytes.ends_with(b"#")) {
            working_set.error(ParseError::Unclosed("r#", span));
            return garbage(working_set, span);
        }
    }

    let (bytes, quoted) = if (bytes.starts_with(b"\"") && bytes.ends_with(b"\"") && bytes.len() > 1)
        || (bytes.starts_with(b"\'") && bytes.ends_with(b"\'") && bytes.len() > 1)
    {
        (&bytes[1..(bytes.len() - 1)], true)
    } else if (bytes.starts_with(b"$\"") && bytes.ends_with(b"\"") && bytes.len() > 2)
        || (bytes.starts_with(b"$\'") && bytes.ends_with(b"\'") && bytes.len() > 2)
    {
        (&bytes[2..(bytes.len() - 1)], true)
    } else {
        (bytes, false)
    };

    if let Ok(token) = String::from_utf8(bytes.into()) {
        trace!("-- found {token}");

        if quoted {
            Expression::new(working_set, Expr::String(token), span, Type::String)
        } else if token.contains(' ') {
            working_set.error(ParseError::Expected("string", span));

            garbage(working_set, span)
        } else {
            Expression::new(working_set, Expr::String(token), span, Type::String)
        }
    } else {
        working_set.error(ParseError::Expected("string", span));
        garbage(working_set, span)
    }
}
