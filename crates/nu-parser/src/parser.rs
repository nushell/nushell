use crate::{
    lex, lite_parse,
    lite_parse::LiteCommand,
    parse_mut,
    type_check::{math_result_type, type_compatible},
    LiteBlock, ParseError, Token, TokenContents,
};

use nu_protocol::{
    ast::{
        Argument, Assignment, Bits, Block, Boolean, Call, CellPath, Comparison, Expr, Expression,
        FullCellPath, ImportPattern, ImportPatternHead, ImportPatternMember, Math, Operator,
        PathMember, Pipeline, RangeInclusion, RangeOperator,
    },
    engine::StateWorkingSet,
    span, BlockId, Flag, PositionalArg, Signature, Span, Spanned, SyntaxShape, Type, Unit, VarId,
    ENV_VARIABLE_ID, IN_VARIABLE_ID,
};

use crate::parse_keywords::{
    parse_alias, parse_def, parse_def_predecl, parse_export_in_block, parse_extern, parse_for,
    parse_hide, parse_let, parse_module, parse_overlay, parse_source, parse_use,
};

use itertools::Itertools;
use log::trace;
use std::{
    collections::{HashMap, HashSet},
    num::ParseIntError,
};

#[cfg(feature = "plugin")]
use crate::parse_keywords::parse_register;

#[derive(Debug, Clone)]
pub enum Import {}

pub fn garbage(span: Span) -> Expression {
    Expression::garbage(span)
}

pub fn garbage_pipeline(spans: &[Span]) -> Pipeline {
    Pipeline::from_vec(vec![garbage(span(spans))])
}

fn is_identifier_byte(b: u8) -> bool {
    b != b'.'
        && b != b'['
        && b != b'('
        && b != b'{'
        && b != b'+'
        && b != b'-'
        && b != b'*'
        && b != b'^'
        && b != b'/'
        && b != b'='
        && b != b'!'
        && b != b'<'
        && b != b'>'
        && b != b'&'
        && b != b'|'
}

pub fn is_math_expression_like(
    working_set: &mut StateWorkingSet,
    span: Span,
    expand_aliases_denylist: &[usize],
) -> bool {
    let bytes = working_set.get_span_contents(span);
    if bytes.is_empty() {
        return false;
    }

    if bytes == b"true" || bytes == b"false" || bytes == b"null" || bytes == b"not" {
        return true;
    }

    let b = bytes[0];

    if b == b'('
        || b == b'{'
        || b == b'['
        || b == b'$'
        || b == b'"'
        || b == b'\''
        || b == b'`'
        || b == b'-'
    {
        return true;
    }

    if parse_number(bytes, span).1.is_none() {
        return true;
    }

    if parse_filesize(working_set, span).1.is_none() {
        return true;
    }

    if parse_duration(working_set, span).1.is_none() {
        return true;
    }

    if parse_datetime(working_set, span).1.is_none() {
        return true;
    }

    if parse_binary(working_set, span).1.is_none() {
        return true;
    }

    if parse_range(working_set, span, expand_aliases_denylist)
        .1
        .is_none()
    {
        return true;
    }

    false
}

fn is_identifier(bytes: &[u8]) -> bool {
    bytes.iter().all(|x| is_identifier_byte(*x))
}

fn is_variable(bytes: &[u8]) -> bool {
    if bytes.len() > 1 && bytes[0] == b'$' {
        is_identifier(&bytes[1..])
    } else {
        is_identifier(bytes)
    }
}

pub fn trim_quotes(bytes: &[u8]) -> &[u8] {
    if (bytes.starts_with(b"\"") && bytes.ends_with(b"\"") && bytes.len() > 1)
        || (bytes.starts_with(b"\'") && bytes.ends_with(b"\'") && bytes.len() > 1)
        || (bytes.starts_with(b"`") && bytes.ends_with(b"`") && bytes.len() > 1)
    {
        &bytes[1..(bytes.len() - 1)]
    } else {
        bytes
    }
}

pub fn trim_quotes_str(s: &str) -> &str {
    if (s.starts_with('"') && s.ends_with('"') && s.len() > 1)
        || (s.starts_with('\'') && s.ends_with('\'') && s.len() > 1)
        || (s.starts_with('`') && s.ends_with('`') && s.len() > 1)
    {
        &s[1..(s.len() - 1)]
    } else {
        s
    }
}

pub fn check_call(command: Span, sig: &Signature, call: &Call) -> Option<ParseError> {
    // Allow the call to pass if they pass in the help flag
    if call.named_iter().any(|(n, _, _)| n.item == "help") {
        return None;
    }

    if call.positional_len() < sig.required_positional.len() {
        // Comparing the types of all signature positional arguments against the parsed
        // expressions found in the call. If one type is not found then it could be assumed
        // that that positional argument is missing from the parsed call
        for argument in &sig.required_positional {
            let found = call.positional_iter().fold(false, |ac, expr| {
                if argument.shape.to_type() == expr.ty || argument.shape == SyntaxShape::Any {
                    true
                } else {
                    ac
                }
            });
            if !found {
                if let Some(last) = call.positional_iter().last() {
                    return Some(ParseError::MissingPositional(
                        argument.name.clone(),
                        Span {
                            start: last.span.end,
                            end: last.span.end,
                        },
                        sig.call_signature(),
                    ));
                } else {
                    return Some(ParseError::MissingPositional(
                        argument.name.clone(),
                        Span {
                            start: command.end,
                            end: command.end,
                        },
                        sig.call_signature(),
                    ));
                }
            }
        }

        let missing = &sig.required_positional[call.positional_len()];
        if let Some(last) = call.positional_iter().last() {
            Some(ParseError::MissingPositional(
                missing.name.clone(),
                Span {
                    start: last.span.end,
                    end: last.span.end,
                },
                sig.call_signature(),
            ))
        } else {
            Some(ParseError::MissingPositional(
                missing.name.clone(),
                Span {
                    start: command.end,
                    end: command.end,
                },
                sig.call_signature(),
            ))
        }
    } else {
        for req_flag in sig.named.iter().filter(|x| x.required) {
            if call.named_iter().all(|(n, _, _)| n.item != req_flag.long) {
                return Some(ParseError::MissingRequiredFlag(
                    req_flag.long.clone(),
                    command,
                ));
            }
        }
        None
    }
}

pub fn check_name<'a>(
    working_set: &mut StateWorkingSet,
    spans: &'a [Span],
) -> Option<(&'a Span, ParseError)> {
    let command_len = if !spans.is_empty() {
        if working_set.get_span_contents(spans[0]) == b"export" {
            2
        } else {
            1
        }
    } else {
        return None;
    };

    if spans.len() == 1 {
        None
    } else if spans.len() < command_len + 3 {
        if working_set.get_span_contents(spans[command_len]) == b"=" {
            let name =
                String::from_utf8_lossy(working_set.get_span_contents(span(&spans[..command_len])));
            Some((
                &spans[command_len],
                ParseError::AssignmentMismatch(
                    format!("{} missing name", name),
                    "missing name".into(),
                    spans[command_len],
                ),
            ))
        } else {
            None
        }
    } else if working_set.get_span_contents(spans[command_len + 1]) != b"=" {
        let name =
            String::from_utf8_lossy(working_set.get_span_contents(span(&spans[..command_len])));
        Some((
            &spans[command_len + 1],
            ParseError::AssignmentMismatch(
                format!("{} missing sign", name),
                "missing equal sign".into(),
                spans[command_len + 1],
            ),
        ))
    } else {
        None
    }
}

pub fn parse_external_call(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    trace!("parse external");

    let mut args = vec![];

    let head_contents = working_set.get_span_contents(spans[0]);

    let head_span = if head_contents.starts_with(b"^") {
        Span {
            start: spans[0].start + 1,
            end: spans[0].end,
        }
    } else {
        spans[0]
    };

    let head_contents = working_set.get_span_contents(head_span).to_vec();

    let mut error = None;

    let head = if head_contents.starts_with(b"$") || head_contents.starts_with(b"(") {
        let (arg, err) = parse_expression(working_set, &[head_span], expand_aliases_denylist);
        error = error.or(err);
        Box::new(arg)
    } else {
        let (contents, err) = unescape_unquote_string(&head_contents, head_span);
        error = error.or(err);

        Box::new(Expression {
            expr: Expr::String(contents),
            span: head_span,
            ty: Type::String,
            custom_completion: None,
        })
    };

    for span in &spans[1..] {
        let contents = working_set.get_span_contents(*span);

        if contents.starts_with(b"$") || contents.starts_with(b"(") {
            let (arg, err) = parse_dollar_expr(working_set, *span, expand_aliases_denylist);
            error = error.or(err);
            args.push(arg);
        } else if contents.starts_with(b"[") {
            let (arg, err) = parse_list_expression(
                working_set,
                *span,
                &SyntaxShape::Any,
                expand_aliases_denylist,
            );
            error = error.or(err);
            args.push(arg);
        } else {
            // Eval stage trims the quotes, so we don't have to do the same thing when parsing.
            let contents = if contents.starts_with(b"\"") {
                let (contents, err) = unescape_string(contents, *span);
                error = error.or(err);
                String::from_utf8_lossy(&contents).to_string()
            } else {
                String::from_utf8_lossy(contents).to_string()
            };

            args.push(Expression {
                expr: Expr::String(contents),
                span: *span,
                ty: Type::String,
                custom_completion: None,
            })
        }
    }
    (
        Expression {
            expr: Expr::ExternalCall(head, args),
            span: span(spans),
            ty: Type::Any,
            custom_completion: None,
        },
        error,
    )
}

fn parse_long_flag(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    spans_idx: &mut usize,
    sig: &Signature,
    expand_aliases_denylist: &[usize],
) -> (
    Option<Spanned<String>>,
    Option<Expression>,
    Option<ParseError>,
) {
    let arg_span = spans[*spans_idx];
    let arg_contents = working_set.get_span_contents(arg_span);

    if arg_contents.starts_with(b"--") {
        // FIXME: only use the first flag you find?
        let split: Vec<_> = arg_contents.split(|x| *x == b'=').collect();
        let long_name = String::from_utf8(split[0].into());
        if let Ok(long_name) = long_name {
            let long_name = long_name[2..].to_string();
            if let Some(flag) = sig.get_long_flag(&long_name) {
                if let Some(arg_shape) = &flag.arg {
                    if split.len() > 1 {
                        // and we also have the argument
                        let long_name_len = long_name.len();
                        let mut span = arg_span;
                        span.start += long_name_len + 3; //offset by long flag and '='

                        let (arg, err) =
                            parse_value(working_set, span, arg_shape, expand_aliases_denylist);

                        (
                            Some(Spanned {
                                item: long_name,
                                span: Span {
                                    start: arg_span.start,
                                    end: arg_span.start + long_name_len + 2,
                                },
                            }),
                            Some(arg),
                            err,
                        )
                    } else if let Some(arg) = spans.get(*spans_idx + 1) {
                        let (arg, err) =
                            parse_value(working_set, *arg, arg_shape, expand_aliases_denylist);

                        *spans_idx += 1;
                        (
                            Some(Spanned {
                                item: long_name,
                                span: arg_span,
                            }),
                            Some(arg),
                            err,
                        )
                    } else {
                        (
                            Some(Spanned {
                                item: long_name,
                                span: arg_span,
                            }),
                            None,
                            Some(ParseError::MissingFlagParam(
                                arg_shape.to_string(),
                                arg_span,
                            )),
                        )
                    }
                } else {
                    // A flag with no argument
                    (
                        Some(Spanned {
                            item: long_name,
                            span: arg_span,
                        }),
                        None,
                        None,
                    )
                }
            } else {
                (
                    Some(Spanned {
                        item: long_name.clone(),
                        span: arg_span,
                    }),
                    None,
                    Some(ParseError::UnknownFlag(
                        sig.name.clone(),
                        long_name.clone(),
                        arg_span,
                    )),
                )
            }
        } else {
            (
                Some(Spanned {
                    item: "--".into(),
                    span: arg_span,
                }),
                None,
                Some(ParseError::NonUtf8(arg_span)),
            )
        }
    } else {
        (None, None, None)
    }
}

fn parse_short_flags(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    spans_idx: &mut usize,
    positional_idx: usize,
    sig: &Signature,
) -> (Option<Vec<Flag>>, Option<ParseError>) {
    let mut error = None;
    let arg_span = spans[*spans_idx];

    let arg_contents = working_set.get_span_contents(arg_span);

    if arg_contents.starts_with(b"-") && arg_contents.len() > 1 {
        let short_flags = &arg_contents[1..];
        let mut found_short_flags = vec![];
        let mut unmatched_short_flags = vec![];
        for short_flag in short_flags.iter().enumerate() {
            let short_flag_char = char::from(*short_flag.1);
            let orig = arg_span;
            let short_flag_span = Span {
                start: orig.start + 1 + short_flag.0,
                end: orig.start + 1 + short_flag.0 + 1,
            };
            if let Some(flag) = sig.get_short_flag(short_flag_char) {
                // If we require an arg and are in a batch of short flags, error
                if !found_short_flags.is_empty() && flag.arg.is_some() {
                    error = error.or(Some(ParseError::ShortFlagBatchCantTakeArg(short_flag_span)))
                }
                found_short_flags.push(flag);
            } else {
                unmatched_short_flags.push(short_flag_span);
            }
        }

        if found_short_flags.is_empty() {
            // check to see if we have a negative number
            if let Some(positional) = sig.get_positional(positional_idx) {
                if positional.shape == SyntaxShape::Int || positional.shape == SyntaxShape::Number {
                    if String::from_utf8_lossy(arg_contents).parse::<f64>().is_ok() {
                        return (None, None);
                    } else if let Some(first) = unmatched_short_flags.first() {
                        let contents = working_set.get_span_contents(*first);
                        error = error.or_else(|| {
                            Some(ParseError::UnknownFlag(
                                sig.name.clone(),
                                format!("-{}", String::from_utf8_lossy(contents)),
                                *first,
                            ))
                        });
                    }
                } else if let Some(first) = unmatched_short_flags.first() {
                    let contents = working_set.get_span_contents(*first);
                    error = error.or_else(|| {
                        Some(ParseError::UnknownFlag(
                            sig.name.clone(),
                            format!("-{}", String::from_utf8_lossy(contents)),
                            *first,
                        ))
                    });
                }
            } else if let Some(first) = unmatched_short_flags.first() {
                let contents = working_set.get_span_contents(*first);
                error = error.or_else(|| {
                    Some(ParseError::UnknownFlag(
                        sig.name.clone(),
                        format!("-{}", String::from_utf8_lossy(contents)),
                        *first,
                    ))
                });
            }
        } else if !unmatched_short_flags.is_empty() {
            if let Some(first) = unmatched_short_flags.first() {
                let contents = working_set.get_span_contents(*first);
                error = error.or_else(|| {
                    Some(ParseError::UnknownFlag(
                        sig.name.clone(),
                        format!("-{}", String::from_utf8_lossy(contents)),
                        *first,
                    ))
                });
            }
        }

        (Some(found_short_flags), error)
    } else {
        (None, None)
    }
}

fn first_kw_idx(
    working_set: &StateWorkingSet,
    signature: &Signature,
    spans: &[Span],
    spans_idx: usize,
    positional_idx: usize,
) -> (Option<usize>, usize) {
    for idx in (positional_idx + 1)..signature.num_positionals() {
        if let Some(PositionalArg {
            shape: SyntaxShape::Keyword(kw, ..),
            ..
        }) = signature.get_positional(idx)
        {
            #[allow(clippy::needless_range_loop)]
            for span_idx in spans_idx..spans.len() {
                let contents = working_set.get_span_contents(spans[span_idx]);

                if contents == kw {
                    return (Some(idx), span_idx);
                }
            }
        }
    }
    (None, spans.len())
}

fn calculate_end_span(
    working_set: &StateWorkingSet,
    signature: &Signature,
    spans: &[Span],
    spans_idx: usize,
    positional_idx: usize,
) -> usize {
    if signature.rest_positional.is_some() {
        spans.len()
    } else {
        let (kw_pos, kw_idx) =
            first_kw_idx(working_set, signature, spans, spans_idx, positional_idx);

        if let Some(kw_pos) = kw_pos {
            // We found a keyword. Keywords, once found, create a guidepost to
            // show us where the positionals will lay into the arguments. Because they're
            // keywords, they get to set this by being present

            let positionals_between = kw_pos - positional_idx - 1;
            if positionals_between > (kw_idx - spans_idx) {
                kw_idx
            } else {
                kw_idx - positionals_between
            }
        } else {
            // Make space for the remaining require positionals, if we can
            if signature.num_positionals_after(positional_idx) == 0 {
                spans.len()
            } else if positional_idx < signature.required_positional.len()
                && spans.len() > (signature.required_positional.len() - positional_idx)
            {
                spans.len() - (signature.required_positional.len() - positional_idx - 1)
            } else {
                spans_idx + 1
            }
        }
    }
}

pub fn parse_multispan_value(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    spans_idx: &mut usize,
    shape: &SyntaxShape,
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    let mut error = None;

    match shape {
        SyntaxShape::VarWithOptType => {
            trace!("parsing: var with opt type");

            let (arg, err) = parse_var_with_opt_type(working_set, spans, spans_idx, false);
            error = error.or(err);

            (arg, error)
        }
        SyntaxShape::RowCondition => {
            trace!("parsing: row condition");
            let (arg, err) =
                parse_row_condition(working_set, &spans[*spans_idx..], expand_aliases_denylist);
            error = error.or(err);
            *spans_idx = spans.len() - 1;

            (arg, error)
        }
        SyntaxShape::MathExpression => {
            trace!("parsing: math expression");

            let (arg, err) = parse_math_expression(
                working_set,
                &spans[*spans_idx..],
                None,
                expand_aliases_denylist,
            );
            error = error.or(err);
            *spans_idx = spans.len() - 1;

            (arg, error)
        }
        SyntaxShape::Expression => {
            trace!("parsing: expression");

            let (arg, err) =
                parse_expression(working_set, &spans[*spans_idx..], expand_aliases_denylist);
            error = error.or(err);
            *spans_idx = spans.len() - 1;

            (arg, error)
        }
        SyntaxShape::ImportPattern => {
            trace!("parsing: import pattern");

            let (arg, err) =
                parse_import_pattern(working_set, &spans[*spans_idx..], expand_aliases_denylist);
            error = error.or(err);
            *spans_idx = spans.len() - 1;

            (arg, error)
        }
        SyntaxShape::Keyword(keyword, arg) => {
            trace!(
                "parsing: keyword({}) {:?}",
                String::from_utf8_lossy(keyword),
                arg
            );
            let arg_span = spans[*spans_idx];

            let arg_contents = working_set.get_span_contents(arg_span);

            if arg_contents != keyword {
                // When keywords mismatch, this is a strong indicator of something going wrong.
                // We won't often override the current error, but as this is a strong indicator
                // go ahead and override the current error and tell the user about the missing
                // keyword/literal.
                error = Some(ParseError::ExpectedKeyword(
                    String::from_utf8_lossy(keyword).into(),
                    arg_span,
                ))
            }

            *spans_idx += 1;
            if *spans_idx >= spans.len() {
                error = error.or_else(|| {
                    Some(ParseError::KeywordMissingArgument(
                        arg.to_string(),
                        String::from_utf8_lossy(keyword).into(),
                        Span {
                            start: spans[*spans_idx - 1].end,
                            end: spans[*spans_idx - 1].end,
                        },
                    ))
                });
                return (
                    Expression {
                        expr: Expr::Keyword(
                            keyword.clone(),
                            spans[*spans_idx - 1],
                            Box::new(Expression::garbage(arg_span)),
                        ),
                        span: arg_span,
                        ty: Type::Any,
                        custom_completion: None,
                    },
                    error,
                );
            }
            let keyword_span = spans[*spans_idx - 1];
            let (expr, err) =
                parse_multispan_value(working_set, spans, spans_idx, arg, expand_aliases_denylist);
            error = error.or(err);
            let ty = expr.ty.clone();

            (
                Expression {
                    expr: Expr::Keyword(keyword.clone(), keyword_span, Box::new(expr)),
                    span: arg_span,
                    ty,
                    custom_completion: None,
                },
                error,
            )
        }
        _ => {
            // All other cases are single-span values
            let arg_span = spans[*spans_idx];

            let (arg, err) = parse_value(working_set, arg_span, shape, expand_aliases_denylist);
            error = error.or(err);

            (arg, error)
        }
    }
}

pub struct ParsedInternalCall {
    pub call: Box<Call>,
    pub output: Type,
    pub error: Option<ParseError>,
}

pub fn parse_internal_call(
    working_set: &mut StateWorkingSet,
    command_span: Span,
    spans: &[Span],
    decl_id: usize,
    expand_aliases_denylist: &[usize],
) -> ParsedInternalCall {
    trace!("parsing: internal call (decl id: {})", decl_id);

    let mut error = None;

    let mut call = Call::new(command_span);
    call.decl_id = decl_id;
    call.head = command_span;

    let decl = working_set.get_decl(decl_id);
    let signature = decl.signature();
    let output = signature.output_type.clone();

    working_set.type_scope.add_type(output.clone());

    if signature.creates_scope {
        working_set.enter_scope();
    }

    // The index into the positional parameter in the definition
    let mut positional_idx = 0;

    // The index into the spans of argument data given to parse
    // Starting at the first argument
    let mut spans_idx = 0;

    while spans_idx < spans.len() {
        let arg_span = spans[spans_idx];

        // Check if we're on a long flag, if so, parse
        let (long_name, arg, err) = parse_long_flag(
            working_set,
            spans,
            &mut spans_idx,
            &signature,
            expand_aliases_denylist,
        );
        if let Some(long_name) = long_name {
            // We found a long flag, like --bar
            error = error.or(err);
            call.add_named((long_name, None, arg));
            spans_idx += 1;
            continue;
        }

        // Check if we're on a short flag or group of short flags, if so, parse
        let (short_flags, err) = parse_short_flags(
            working_set,
            spans,
            &mut spans_idx,
            positional_idx,
            &signature,
        );

        if let Some(mut short_flags) = short_flags {
            if short_flags.is_empty() {
                short_flags.push(Flag {
                    long: "".to_string(),
                    short: Some('a'),
                    arg: None,
                    required: false,
                    desc: "".to_string(),
                    var_id: None,
                    default_value: None,
                })
            }
            error = error.or(err);
            for flag in short_flags {
                if let Some(arg_shape) = flag.arg {
                    if let Some(arg) = spans.get(spans_idx + 1) {
                        let (arg, err) =
                            parse_value(working_set, *arg, &arg_shape, expand_aliases_denylist);
                        error = error.or(err);

                        if flag.long.is_empty() {
                            if let Some(short) = flag.short {
                                call.add_named((
                                    Spanned {
                                        item: String::new(),
                                        span: spans[spans_idx],
                                    },
                                    Some(Spanned {
                                        item: short.to_string(),
                                        span: spans[spans_idx],
                                    }),
                                    Some(arg),
                                ));
                            }
                        } else {
                            call.add_named((
                                Spanned {
                                    item: flag.long.clone(),
                                    span: spans[spans_idx],
                                },
                                None,
                                Some(arg),
                            ));
                        }
                        spans_idx += 1;
                    } else {
                        error = error.or_else(|| {
                            Some(ParseError::MissingFlagParam(
                                arg_shape.to_string(),
                                arg_span,
                            ))
                        })
                    }
                } else if flag.long.is_empty() {
                    if let Some(short) = flag.short {
                        call.add_named((
                            Spanned {
                                item: String::new(),
                                span: spans[spans_idx],
                            },
                            Some(Spanned {
                                item: short.to_string(),
                                span: spans[spans_idx],
                            }),
                            None,
                        ));
                    }
                } else {
                    call.add_named((
                        Spanned {
                            item: flag.long.clone(),
                            span: spans[spans_idx],
                        },
                        None,
                        None,
                    ));
                }
            }
            spans_idx += 1;
            continue;
        }

        // Parse a positional arg if there is one
        if let Some(positional) = signature.get_positional(positional_idx) {
            let end = calculate_end_span(working_set, &signature, spans, spans_idx, positional_idx);

            let end = if spans.len() > spans_idx && end == spans_idx {
                end + 1
            } else {
                end
            };

            if spans[..end].is_empty() || spans_idx == end {
                error = error.or_else(|| {
                    Some(ParseError::MissingPositional(
                        positional.name.clone(),
                        Span {
                            start: spans[spans_idx].end,
                            end: spans[spans_idx].end,
                        },
                        signature.call_signature(),
                    ))
                });
                positional_idx += 1;
                continue;
            }

            let orig_idx = spans_idx;
            let (arg, err) = parse_multispan_value(
                working_set,
                &spans[..end],
                &mut spans_idx,
                &positional.shape,
                expand_aliases_denylist,
            );
            error = error.or(err);

            let arg = if !type_compatible(&positional.shape.to_type(), &arg.ty) {
                let span = span(&spans[orig_idx..spans_idx]);
                error = error.or_else(|| {
                    Some(ParseError::TypeMismatch(
                        positional.shape.to_type(),
                        arg.ty,
                        arg.span,
                    ))
                });
                Expression::garbage(span)
            } else {
                arg
            };
            call.add_positional(arg);
            positional_idx += 1;
        } else {
            call.add_positional(Expression::garbage(arg_span));
            error = error.or_else(|| {
                Some(ParseError::ExtraPositional(
                    signature.call_signature(),
                    arg_span,
                ))
            })
        }

        error = error.or(err);
        spans_idx += 1;
    }

    let err = check_call(command_span, &signature, &call);
    error = error.or(err);

    if signature.creates_scope {
        working_set.exit_scope();
    }

    ParsedInternalCall {
        call: Box::new(call),
        output,
        error,
    }
}

pub fn parse_call(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    head: Span,
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    trace!("parsing: call");

    if spans.is_empty() {
        return (
            garbage(head),
            Some(ParseError::UnknownState(
                "Encountered command with zero spans".into(),
                span(spans),
            )),
        );
    }

    let mut pos = 0;
    let cmd_start = pos;
    let mut name_spans = vec![];
    let mut name = vec![];

    for word_span in spans[cmd_start..].iter() {
        // Find the longest group of words that could form a command

        if is_math_expression_like(working_set, *word_span, expand_aliases_denylist) {
            let bytes = working_set.get_span_contents(*word_span);
            if bytes != b"true" && bytes != b"false" && bytes != b"null" && bytes != b"not" {
                break;
            }
        }

        name_spans.push(*word_span);

        let name_part = working_set.get_span_contents(*word_span);
        if name.is_empty() {
            name.extend(name_part);
        } else {
            name.push(b' ');
            name.extend(name_part);
        }

        // If the word is an alias, expand it and re-parse the expression
        if let Some(alias_id) = working_set.find_alias(&name) {
            if !expand_aliases_denylist.contains(&alias_id) {
                trace!("expanding alias");

                let expansion = working_set.get_alias(alias_id);

                let expansion_span = span(expansion);

                let orig_span = span(&[spans[cmd_start], spans[pos]]);
                let mut new_spans: Vec<Span> = vec![];
                new_spans.extend(&spans[0..cmd_start]);
                new_spans.extend(expansion);
                // TODO: This seems like it should be `pos + 1`. `pos` starts as 0
                if spans.len() > pos {
                    new_spans.extend(&spans[(pos + 1)..]);
                }

                let mut expand_aliases_denylist = expand_aliases_denylist.to_vec();
                expand_aliases_denylist.push(alias_id);

                let lite_command = LiteCommand {
                    comments: vec![],
                    parts: new_spans.clone(),
                };

                let (mut result, err) =
                    parse_builtin_commands(working_set, &lite_command, &expand_aliases_denylist);

                let mut result = result.expressions.remove(0);

                result.replace_span(working_set, expansion_span, orig_span);

                return (result, err);
            }
        }

        pos += 1;
    }

    let input = working_set.type_scope.get_previous();
    let mut maybe_decl_id = working_set.find_decl(&name, input);

    while maybe_decl_id.is_none() {
        // Find the longest command match
        if name_spans.len() <= 1 {
            // Keep the first word even if it does not match -- could be external command
            break;
        }

        name_spans.pop();
        pos -= 1;

        let mut name = vec![];
        for name_span in &name_spans {
            let name_part = working_set.get_span_contents(*name_span);
            if name.is_empty() {
                name.extend(name_part);
            } else {
                name.push(b' ');
                name.extend(name_part);
            }
        }
        maybe_decl_id = working_set.find_decl(&name, input);
    }

    if let Some(decl_id) = maybe_decl_id {
        // Before the internal parsing we check if there is no let or alias declarations
        // that are missing their name, e.g.: let = 1 or alias = 2
        if spans.len() > 1 {
            let test_equal = working_set.get_span_contents(spans[1]);

            if test_equal == [b'='] {
                trace!("incomplete statement");

                return (
                    garbage(span(spans)),
                    Some(ParseError::UnknownState(
                        "Incomplete statement".into(),
                        span(spans),
                    )),
                );
            }
        }

        trace!("parsing: internal call");

        // parse internal command
        let parsed_call = parse_internal_call(
            working_set,
            span(&spans[cmd_start..pos]),
            &spans[pos..],
            decl_id,
            expand_aliases_denylist,
        );

        (
            Expression {
                expr: Expr::Call(parsed_call.call),
                span: span(spans),
                ty: parsed_call.output,
                custom_completion: None,
            },
            parsed_call.error,
        )
    } else {
        // We might be parsing left-unbounded range ("..10")
        let bytes = working_set.get_span_contents(spans[0]);
        trace!("parsing: range {:?} ", bytes);
        if let (Some(b'.'), Some(b'.')) = (bytes.first(), bytes.get(1)) {
            trace!("-- found leading range indicator");
            let (range_expr, range_err) =
                parse_range(working_set, spans[0], expand_aliases_denylist);
            if range_err.is_none() {
                trace!("-- successfully parsed range");
                return (range_expr, range_err);
            }
        }
        trace!("parsing: external call");

        // Otherwise, try external command
        parse_external_call(working_set, spans, expand_aliases_denylist)
    }
}

pub fn parse_binary(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Expression, Option<ParseError>) {
    let (hex_value, err) = parse_binary_with_base(working_set, span, 16, 2, b"0x[", b"]");
    if err.is_some() {
        let (octal_value, err) = parse_binary_with_base(working_set, span, 8, 3, b"0o[", b"]");
        if err.is_some() {
            return parse_binary_with_base(working_set, span, 2, 8, b"0b[", b"]");
        }
        return (octal_value, err);
    }
    (hex_value, err)
}

fn parse_binary_with_base(
    working_set: &mut StateWorkingSet,
    span: Span,
    base: u32,
    min_digits_per_byte: usize,
    prefix: &[u8],
    suffix: &[u8],
) -> (Expression, Option<ParseError>) {
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

            let mut binary_value = vec![];
            for token in lexed {
                match token.contents {
                    TokenContents::Item => {
                        let contents = working_set.get_span_contents(token.span);

                        binary_value.extend_from_slice(contents);
                    }
                    TokenContents::Pipe => {
                        return (
                            garbage(span),
                            Some(ParseError::Expected("binary".into(), span)),
                        );
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
                    return (
                        Expression {
                            expr: Expr::Binary(v),
                            span,
                            ty: Type::Binary,
                            custom_completion: None,
                        },
                        err,
                    )
                }
                Err(x) => {
                    return (
                        garbage(span),
                        Some(ParseError::IncorrectValue(
                            "not a binary value".into(),
                            span,
                            x.to_string(),
                        )),
                    )
                }
            }
        }
    }
    (
        garbage(span),
        Some(ParseError::Expected("binary".into(), span)),
    )
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

pub fn parse_int(token: &[u8], span: Span) -> (Expression, Option<ParseError>) {
    if let Some(token) = token.strip_prefix(b"0x") {
        if let Ok(v) = i64::from_str_radix(&String::from_utf8_lossy(token), 16) {
            (
                Expression {
                    expr: Expr::Int(v),
                    span,
                    ty: Type::Int,
                    custom_completion: None,
                },
                None,
            )
        } else {
            (
                garbage(span),
                Some(ParseError::Mismatch(
                    "int".into(),
                    "incompatible int".into(),
                    span,
                )),
            )
        }
    } else if let Some(token) = token.strip_prefix(b"0b") {
        if let Ok(v) = i64::from_str_radix(&String::from_utf8_lossy(token), 2) {
            (
                Expression {
                    expr: Expr::Int(v),
                    span,
                    ty: Type::Int,
                    custom_completion: None,
                },
                None,
            )
        } else {
            (
                garbage(span),
                Some(ParseError::Mismatch(
                    "int".into(),
                    "incompatible int".into(),
                    span,
                )),
            )
        }
    } else if let Some(token) = token.strip_prefix(b"0o") {
        if let Ok(v) = i64::from_str_radix(&String::from_utf8_lossy(token), 8) {
            (
                Expression {
                    expr: Expr::Int(v),
                    span,
                    ty: Type::Int,
                    custom_completion: None,
                },
                None,
            )
        } else {
            (
                garbage(span),
                Some(ParseError::Mismatch(
                    "int".into(),
                    "incompatible int".into(),
                    span,
                )),
            )
        }
    } else if let Ok(x) = String::from_utf8_lossy(token).parse::<i64>() {
        (
            Expression {
                expr: Expr::Int(x),
                span,
                ty: Type::Int,
                custom_completion: None,
            },
            None,
        )
    } else {
        (
            garbage(span),
            Some(ParseError::Expected("int".into(), span)),
        )
    }
}

pub fn parse_float(token: &[u8], span: Span) -> (Expression, Option<ParseError>) {
    if let Ok(x) = String::from_utf8_lossy(token).parse::<f64>() {
        (
            Expression {
                expr: Expr::Float(x),
                span,
                ty: Type::Float,
                custom_completion: None,
            },
            None,
        )
    } else {
        (
            garbage(span),
            Some(ParseError::Expected("float".into(), span)),
        )
    }
}

pub fn parse_number(token: &[u8], span: Span) -> (Expression, Option<ParseError>) {
    if let (x, None) = parse_int(token, span) {
        (x, None)
    } else if let (x, None) = parse_float(token, span) {
        (x, None)
    } else {
        (
            garbage(span),
            Some(ParseError::Expected("number".into(), span)),
        )
    }
}

pub fn parse_range(
    working_set: &mut StateWorkingSet,
    span: Span,
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    trace!("parsing: range");

    // Range follows the following syntax: [<from>][<next_operator><next>]<range_operator>[<to>]
    //   where <next_operator> is ".."
    //   and  <range_operator> is ".." or "..<"
    //   and one of the <from> or <to> bounds must be present (just '..' is not allowed since it
    //     looks like parent directory)

    let contents = working_set.get_span_contents(span);

    let token = if let Ok(s) = String::from_utf8(contents.into()) {
        s
    } else {
        return (garbage(span), Some(ParseError::NonUtf8(span)));
    };

    if !token.contains("..") {
        return (
            garbage(span),
            Some(ParseError::Expected(
                "at least one range bound set".into(),
                span,
            )),
        );
    }

    // First, figure out what exact operators are used and determine their positions
    let dotdot_pos: Vec<_> = token.match_indices("..").map(|(pos, _)| pos).collect();

    let (next_op_pos, range_op_pos) =
        match dotdot_pos.len() {
            1 => (None, dotdot_pos[0]),
            2 => (Some(dotdot_pos[0]), dotdot_pos[1]),
            _ => return (
                garbage(span),
                Some(ParseError::Expected(
                    "one range operator ('..' or '..<') and optionally one next operator ('..')"
                        .into(),
                    span,
                )),
            ),
        };

    let (inclusion, range_op_str, range_op_span) = if let Some(pos) = token.find("..<") {
        if pos == range_op_pos {
            let op_str = "..<";
            let op_span = Span::new(
                span.start + range_op_pos,
                span.start + range_op_pos + op_str.len(),
            );
            (RangeInclusion::RightExclusive, "..<", op_span)
        } else {
            return (
                garbage(span),
                Some(ParseError::Expected(
                    "inclusive operator preceding second range bound".into(),
                    span,
                )),
            );
        }
    } else {
        let op_str = "..";
        let op_span = Span::new(
            span.start + range_op_pos,
            span.start + range_op_pos + op_str.len(),
        );
        (RangeInclusion::Inclusive, "..", op_span)
    };

    // Now, based on the operator positions, figure out where the bounds & next are located and
    // parse them
    // TODO: Actually parse the next number in the range
    let from = if token.starts_with("..") {
        // token starts with either next operator, or range operator -- we don't care which one
        None
    } else {
        let from_span = Span::new(span.start, span.start + dotdot_pos[0]);
        match parse_value(
            working_set,
            from_span,
            &SyntaxShape::Number,
            expand_aliases_denylist,
        ) {
            (expression, None) => Some(Box::new(expression)),
            _ => {
                return (
                    garbage(span),
                    Some(ParseError::Expected("number".into(), span)),
                )
            }
        }
    };

    let to = if token.ends_with(range_op_str) {
        None
    } else {
        let to_span = Span::new(range_op_span.end, span.end);
        match parse_value(
            working_set,
            to_span,
            &SyntaxShape::Number,
            expand_aliases_denylist,
        ) {
            (expression, None) => Some(Box::new(expression)),
            _ => {
                return (
                    garbage(span),
                    Some(ParseError::Expected("number".into(), span)),
                )
            }
        }
    };

    trace!("-- from: {:?} to: {:?}", from, to);

    if let (None, None) = (&from, &to) {
        return (
            garbage(span),
            Some(ParseError::Expected(
                "at least one range bound set".into(),
                span,
            )),
        );
    }

    let (next, next_op_span) = if let Some(pos) = next_op_pos {
        let next_op_span = Span::new(span.start + pos, span.start + pos + "..".len());
        let next_span = Span::new(next_op_span.end, range_op_span.start);

        match parse_value(
            working_set,
            next_span,
            &SyntaxShape::Number,
            expand_aliases_denylist,
        ) {
            (expression, None) => (Some(Box::new(expression)), next_op_span),
            _ => {
                return (
                    garbage(span),
                    Some(ParseError::Expected("number".into(), span)),
                )
            }
        }
    } else {
        (None, span)
    };

    let range_op = RangeOperator {
        inclusion,
        span: range_op_span,
        next_op_span,
    };

    (
        Expression {
            expr: Expr::Range(from, next, to, range_op),
            span,
            ty: Type::Range,
            custom_completion: None,
        },
        None,
    )
}

pub(crate) fn parse_dollar_expr(
    working_set: &mut StateWorkingSet,
    span: Span,
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    trace!("parsing: dollar expression");
    let contents = working_set.get_span_contents(span);

    if contents.starts_with(b"$\"") || contents.starts_with(b"$'") {
        parse_string_interpolation(working_set, span, expand_aliases_denylist)
    } else if let (expr, None) = parse_range(working_set, span, expand_aliases_denylist) {
        (expr, None)
    } else {
        parse_full_cell_path(working_set, None, span, expand_aliases_denylist)
    }
}

pub fn parse_string_interpolation(
    working_set: &mut StateWorkingSet,
    span: Span,
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    #[derive(PartialEq, Eq, Debug)]
    enum InterpolationMode {
        String,
        Expression,
    }
    let mut error = None;

    let contents = working_set.get_span_contents(span);

    let mut double_quote = false;

    let (start, end) = if contents.starts_with(b"$\"") {
        double_quote = true;
        let end = if contents.ends_with(b"\"") && contents.len() > 2 {
            span.end - 1
        } else {
            span.end
        };
        (span.start + 2, end)
    } else if contents.starts_with(b"$'") {
        let end = if contents.ends_with(b"'") && contents.len() > 2 {
            span.end - 1
        } else {
            span.end
        };
        (span.start + 2, end)
    } else {
        (span.start, span.end)
    };

    let inner_span = Span { start, end };
    let contents = working_set.get_span_contents(inner_span).to_vec();

    let mut output = vec![];
    let mut mode = InterpolationMode::String;
    let mut token_start = start;
    let mut delimiter_stack = vec![];

    let mut b = start;

    while b != end {
        if contents[b - start] == b'('
            && (if double_quote && (b - start) > 0 {
                contents[b - start - 1] != b'\\'
            } else {
                true
            })
            && mode == InterpolationMode::String
        {
            mode = InterpolationMode::Expression;
            if token_start < b {
                let span = Span {
                    start: token_start,
                    end: b,
                };
                let str_contents = working_set.get_span_contents(span);

                let str_contents = if double_quote {
                    let (str_contents, err) = unescape_string(str_contents, span);
                    error = error.or(err);

                    str_contents
                } else {
                    str_contents.to_vec()
                };

                output.push(Expression {
                    expr: Expr::String(String::from_utf8_lossy(&str_contents).to_string()),
                    span,
                    ty: Type::String,
                    custom_completion: None,
                });
                token_start = b;
            }
        }
        if mode == InterpolationMode::Expression {
            let byte = contents[b - start];
            if let Some(b'\'') = delimiter_stack.last() {
                if byte == b'\'' {
                    delimiter_stack.pop();
                }
            } else if let Some(b'"') = delimiter_stack.last() {
                if byte == b'"' {
                    delimiter_stack.pop();
                }
            } else if let Some(b'`') = delimiter_stack.last() {
                if byte == b'`' {
                    delimiter_stack.pop();
                }
            } else if byte == b'\'' {
                delimiter_stack.push(b'\'')
            } else if byte == b'"' {
                delimiter_stack.push(b'"');
            } else if byte == b'`' {
                delimiter_stack.push(b'`')
            } else if byte == b'(' {
                delimiter_stack.push(b')');
            } else if byte == b')' {
                if let Some(b')') = delimiter_stack.last() {
                    delimiter_stack.pop();
                }
                if delimiter_stack.is_empty() {
                    mode = InterpolationMode::String;

                    if token_start < b {
                        let span = Span {
                            start: token_start,
                            end: b + 1,
                        };

                        let (expr, err) =
                            parse_full_cell_path(working_set, None, span, expand_aliases_denylist);
                        error = error.or(err);
                        output.push(expr);
                    }

                    token_start = b + 1;
                    continue;
                }
            }
        }
        b += 1;
    }

    match mode {
        InterpolationMode::String => {
            if token_start < end {
                let span = Span {
                    start: token_start,
                    end,
                };
                let str_contents = working_set.get_span_contents(span);

                let str_contents = if double_quote {
                    let (str_contents, err) = unescape_string(str_contents, span);
                    error = error.or(err);

                    str_contents
                } else {
                    str_contents.to_vec()
                };

                output.push(Expression {
                    expr: Expr::String(String::from_utf8_lossy(&str_contents).to_string()),
                    span,
                    ty: Type::String,
                    custom_completion: None,
                });
            }
        }
        InterpolationMode::Expression => {
            if token_start < end {
                let span = Span {
                    start: token_start,
                    end,
                };

                let (expr, err) =
                    parse_full_cell_path(working_set, None, span, expand_aliases_denylist);
                error = error.or(err);
                output.push(expr);
            }
        }
    }

    (
        Expression {
            expr: Expr::StringInterpolation(output),
            span,
            ty: Type::String,
            custom_completion: None,
        },
        error,
    )
}

pub fn parse_variable_expr(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Expression, Option<ParseError>) {
    let contents = working_set.get_span_contents(span);

    if contents == b"$nothing" {
        return (
            Expression {
                expr: Expr::Nothing,
                span,
                ty: Type::Nothing,
                custom_completion: None,
            },
            None,
        );
    } else if contents == b"$nu" {
        return (
            Expression {
                expr: Expr::Var(nu_protocol::NU_VARIABLE_ID),
                span,
                ty: Type::Any,
                custom_completion: None,
            },
            None,
        );
    } else if contents == b"$in" {
        return (
            Expression {
                expr: Expr::Var(nu_protocol::IN_VARIABLE_ID),
                span,
                ty: Type::Any,
                custom_completion: None,
            },
            None,
        );
    } else if contents == b"$env" {
        return (
            Expression {
                expr: Expr::Var(nu_protocol::ENV_VARIABLE_ID),
                span,
                ty: Type::Any,
                custom_completion: None,
            },
            None,
        );
    }

    let (id, err) = parse_variable(working_set, span);

    if err.is_none() {
        if let Some(id) = id {
            (
                Expression {
                    expr: Expr::Var(id),
                    span,
                    ty: working_set.get_variable(id).ty.clone(),
                    custom_completion: None,
                },
                None,
            )
        } else {
            (garbage(span), Some(ParseError::VariableNotFound(span)))
        }
    } else {
        (garbage(span), err)
    }
}

pub fn parse_cell_path(
    working_set: &mut StateWorkingSet,
    tokens: impl Iterator<Item = Token>,
    mut expect_dot: bool,
    expand_aliases_denylist: &[usize],
    span: Span,
) -> (Vec<PathMember>, Option<ParseError>) {
    let mut error = None;
    let mut tail = vec![];

    for path_element in tokens {
        let bytes = working_set.get_span_contents(path_element.span);

        if expect_dot {
            expect_dot = false;
            if bytes.len() != 1 || bytes[0] != b'.' {
                error = error.or_else(|| Some(ParseError::Expected('.'.into(), path_element.span)));
            }
        } else {
            expect_dot = true;

            match parse_int(bytes, path_element.span) {
                (
                    Expression {
                        expr: Expr::Int(val),
                        span,
                        ..
                    },
                    None,
                ) => tail.push(PathMember::Int {
                    val: val as usize,
                    span,
                }),
                _ => {
                    let (result, err) =
                        parse_string(working_set, path_element.span, expand_aliases_denylist);
                    error = error.or(err);
                    match result {
                        Expression {
                            expr: Expr::String(string),
                            span,
                            ..
                        } => {
                            tail.push(PathMember::String { val: string, span });
                        }
                        _ => {
                            error =
                                error.or_else(|| Some(ParseError::Expected("string".into(), span)));
                        }
                    }
                }
            }
        }
    }

    (tail, error)
}

pub fn parse_full_cell_path(
    working_set: &mut StateWorkingSet,
    implicit_head: Option<VarId>,
    span: Span,
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    let full_cell_span = span;
    let source = working_set.get_span_contents(span);
    let mut error = None;

    let (tokens, err) = lex(source, span.start, &[b'\n', b'\r'], &[b'.'], true);
    error = error.or(err);

    let mut tokens = tokens.into_iter().peekable();
    if let Some(head) = tokens.peek() {
        let bytes = working_set.get_span_contents(head.span);
        let (head, expect_dot) = if bytes.starts_with(b"(") {
            trace!("parsing: paren-head of full cell path");

            let head_span = head.span;
            let mut start = head.span.start;
            let mut end = head.span.end;

            if bytes.starts_with(b"(") {
                start += 1;
            }
            if bytes.ends_with(b")") {
                end -= 1;
            } else {
                error = error
                    .or_else(|| Some(ParseError::Unclosed(")".into(), Span { start: end, end })));
            }

            let span = Span { start, end };

            let source = working_set.get_span_contents(span);

            let (output, err) = lex(source, span.start, &[b'\n', b'\r'], &[], true);
            error = error.or(err);

            let (output, err) = lite_parse(&output);
            error = error.or(err);

            // Creating a Type scope to parse the new block. This will keep track of
            // the previous input type found in that block
            let (output, err) =
                parse_block(working_set, &output, true, expand_aliases_denylist, true);
            working_set
                .type_scope
                .add_type(working_set.type_scope.get_last_output());

            let ty = output
                .pipelines
                .last()
                .and_then(|Pipeline { expressions, .. }| expressions.last())
                .map(|expr| match expr.expr {
                    Expr::BinaryOp(..) => expr.ty.clone(),
                    _ => working_set.type_scope.get_last_output(),
                })
                .unwrap_or_else(|| working_set.type_scope.get_last_output());

            error = error.or(err);

            let block_id = working_set.add_block(output);
            tokens.next();

            (
                Expression {
                    expr: Expr::Subexpression(block_id),
                    span: head_span,
                    ty,
                    custom_completion: None,
                },
                true,
            )
        } else if bytes.starts_with(b"[") {
            trace!("parsing: table head of full cell path");

            let (output, err) =
                parse_table_expression(working_set, head.span, expand_aliases_denylist);
            error = error.or(err);

            tokens.next();

            (output, true)
        } else if bytes.starts_with(b"{") {
            trace!("parsing: record head of full cell path");
            let (output, err) = parse_record(working_set, head.span, expand_aliases_denylist);
            error = error.or(err);

            tokens.next();

            (output, true)
        } else if bytes.starts_with(b"$") {
            trace!("parsing: $variable head of full cell path");

            let (out, err) = parse_variable_expr(working_set, head.span);
            error = error.or(err);

            tokens.next();

            (out, true)
        } else if let Some(var_id) = implicit_head {
            (
                Expression {
                    expr: Expr::Var(var_id),
                    span: Span::new(0, 0),
                    ty: Type::Any,
                    custom_completion: None,
                },
                false,
            )
        } else {
            return (
                garbage(span),
                Some(ParseError::Mismatch(
                    "variable or subexpression".into(),
                    String::from_utf8_lossy(bytes).to_string(),
                    span,
                )),
            );
        };

        let (tail, err) = parse_cell_path(
            working_set,
            tokens,
            expect_dot,
            expand_aliases_denylist,
            span,
        );
        error = error.or(err);

        if !tail.is_empty() {
            (
                Expression {
                    ty: head.ty.clone(), // FIXME. How to access the last type of tail?
                    expr: Expr::FullCellPath(Box::new(FullCellPath { head, tail })),
                    span: full_cell_span,
                    custom_completion: None,
                },
                error,
            )
        } else {
            let ty = head.ty.clone();
            (
                Expression {
                    expr: Expr::FullCellPath(Box::new(FullCellPath { head, tail })),
                    ty,
                    span: full_cell_span,
                    custom_completion: None,
                },
                error,
            )
        }
    } else {
        (garbage(span), error)
    }
}

pub fn parse_directory(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Expression, Option<ParseError>) {
    let bytes = working_set.get_span_contents(span);
    let (token, err) = unescape_unquote_string(bytes, span);
    trace!("parsing: directory");

    if err.is_none() {
        trace!("-- found {}", token);
        (
            Expression {
                expr: Expr::Directory(token),
                span,
                ty: Type::String,
                custom_completion: None,
            },
            None,
        )
    } else {
        (
            garbage(span),
            Some(ParseError::Expected("directory".into(), span)),
        )
    }
}

pub fn parse_filepath(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Expression, Option<ParseError>) {
    let bytes = working_set.get_span_contents(span);
    let (token, err) = unescape_unquote_string(bytes, span);
    trace!("parsing: filepath");

    if err.is_none() {
        trace!("-- found {}", token);
        (
            Expression {
                expr: Expr::Filepath(token),
                span,
                ty: Type::String,
                custom_completion: None,
            },
            None,
        )
    } else {
        (
            garbage(span),
            Some(ParseError::Expected("filepath".into(), span)),
        )
    }
}

/// Parse a datetime type, eg '2022-02-02'
pub fn parse_datetime(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Expression, Option<ParseError>) {
    trace!("parsing: datetime");

    let bytes = working_set.get_span_contents(span);

    if bytes.is_empty() || !bytes[0].is_ascii_digit() {
        return (
            garbage(span),
            Some(ParseError::Mismatch(
                "datetime".into(),
                "non-datetime".into(),
                span,
            )),
        );
    }

    let token = String::from_utf8_lossy(bytes).to_string();

    if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(&token) {
        return (
            Expression {
                expr: Expr::DateTime(datetime),
                span,
                ty: Type::Date,
                custom_completion: None,
            },
            None,
        );
    }

    // Just the date
    let just_date = token.clone() + "T00:00:00+00:00";
    if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(&just_date) {
        return (
            Expression {
                expr: Expr::DateTime(datetime),
                span,
                ty: Type::Date,
                custom_completion: None,
            },
            None,
        );
    }

    // Date and time, assume UTC
    let datetime = token + "+00:00";
    if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(&datetime) {
        return (
            Expression {
                expr: Expr::DateTime(datetime),
                span,
                ty: Type::Date,
                custom_completion: None,
            },
            None,
        );
    }

    (
        garbage(span),
        Some(ParseError::Mismatch(
            "datetime".into(),
            "non-datetime".into(),
            span,
        )),
    )
}

/// Parse a duration type, eg '10day'
pub fn parse_duration(
    working_set: &StateWorkingSet,
    span: Span,
) -> (Expression, Option<ParseError>) {
    trace!("parsing: duration");

    let bytes = working_set.get_span_contents(span);

    match parse_duration_bytes(bytes, span) {
        Some(expression) => (expression, None),
        None => (
            garbage(span),
            Some(ParseError::Mismatch(
                "duration".into(),
                "non-duration unit".into(),
                span,
            )),
        ),
    }
}

// Borrowed from libm at https://github.com/rust-lang/libm/blob/master/src/math/modf.rs
pub fn modf(x: f64) -> (f64, f64) {
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

pub fn parse_duration_bytes(num_with_unit_bytes: &[u8], span: Span) -> Option<Expression> {
    if num_with_unit_bytes.is_empty()
        || (!num_with_unit_bytes[0].is_ascii_digit() && num_with_unit_bytes[0] != b'-')
    {
        return None;
    }

    let num_with_unit = String::from_utf8_lossy(num_with_unit_bytes).to_string();
    let uppercase_num_with_unit = num_with_unit.to_uppercase();
    let unit_groups = [
        (Unit::Nanosecond, "NS", None),
        (Unit::Microsecond, "US", Some((Unit::Nanosecond, 1000))),
        (Unit::Millisecond, "MS", Some((Unit::Microsecond, 1000))),
        (Unit::Second, "SEC", Some((Unit::Millisecond, 1000))),
        (Unit::Minute, "MIN", Some((Unit::Second, 60))),
        (Unit::Hour, "HR", Some((Unit::Minute, 60))),
        (Unit::Day, "DAY", Some((Unit::Minute, 1440))),
        (Unit::Week, "WK", Some((Unit::Day, 7))),
    ];

    if let Some(unit) = unit_groups
        .iter()
        .find(|&x| uppercase_num_with_unit.ends_with(x.1))
    {
        let mut lhs = num_with_unit;
        for _ in 0..unit.1.len() {
            lhs.pop();
        }

        let (decimal_part, number_part) = modf(match lhs.parse::<f64>() {
            Ok(x) => x,
            Err(_) => return None,
        });

        let (num, unit_to_use) = match unit.2 {
            Some(unit_to_convert_to) => (
                Some(
                    ((number_part * unit_to_convert_to.1 as f64)
                        + (decimal_part * unit_to_convert_to.1 as f64)) as i64,
                ),
                unit_to_convert_to.0,
            ),
            None => (Some(number_part as i64), unit.0),
        };

        if let Some(x) = num {
            trace!("-- found {} {:?}", x, unit_to_use);

            let lhs_span = Span::new(span.start, span.start + lhs.len());
            let unit_span = Span::new(span.start + lhs.len(), span.end);
            return Some(Expression {
                expr: Expr::ValueWithUnit(
                    Box::new(Expression {
                        expr: Expr::Int(x),
                        span: lhs_span,
                        ty: Type::Number,
                        custom_completion: None,
                    }),
                    Spanned {
                        item: unit_to_use,
                        span: unit_span,
                    },
                ),
                span,
                ty: Type::Duration,
                custom_completion: None,
            });
        }
    }

    None
}

/// Parse a unit type, eg '10kb'
pub fn parse_filesize(
    working_set: &StateWorkingSet,
    span: Span,
) -> (Expression, Option<ParseError>) {
    trace!("parsing: filesize");

    let bytes = working_set.get_span_contents(span);

    match parse_filesize_bytes(bytes, span) {
        Some(expression) => (expression, None),
        None => (
            garbage(span),
            Some(ParseError::Mismatch(
                "filesize".into(),
                "non-filesize unit".into(),
                span,
            )),
        ),
    }
}

pub fn parse_filesize_bytes(num_with_unit_bytes: &[u8], span: Span) -> Option<Expression> {
    if num_with_unit_bytes.is_empty()
        || (!num_with_unit_bytes[0].is_ascii_digit() && num_with_unit_bytes[0] != b'-')
    {
        return None;
    }

    let num_with_unit = String::from_utf8_lossy(num_with_unit_bytes).to_string();
    let uppercase_num_with_unit = num_with_unit.to_uppercase();
    let unit_groups = [
        (Unit::Kilobyte, "KB", Some((Unit::Byte, 1000))),
        (Unit::Megabyte, "MB", Some((Unit::Kilobyte, 1000))),
        (Unit::Gigabyte, "GB", Some((Unit::Megabyte, 1000))),
        (Unit::Terabyte, "TB", Some((Unit::Gigabyte, 1000))),
        (Unit::Petabyte, "PB", Some((Unit::Terabyte, 1000))),
        (Unit::Exabyte, "EB", Some((Unit::Petabyte, 1000))),
        (Unit::Zettabyte, "ZB", Some((Unit::Exabyte, 1000))),
        (Unit::Kibibyte, "KIB", Some((Unit::Byte, 1024))),
        (Unit::Mebibyte, "MIB", Some((Unit::Kibibyte, 1024))),
        (Unit::Gibibyte, "GIB", Some((Unit::Mebibyte, 1024))),
        (Unit::Tebibyte, "TIB", Some((Unit::Gibibyte, 1024))),
        (Unit::Pebibyte, "PIB", Some((Unit::Tebibyte, 1024))),
        (Unit::Exbibyte, "EIB", Some((Unit::Pebibyte, 1024))),
        (Unit::Zebibyte, "ZIB", Some((Unit::Exbibyte, 1024))),
        (Unit::Byte, "B", None),
    ];

    if let Some(unit) = unit_groups
        .iter()
        .find(|&x| uppercase_num_with_unit.ends_with(x.1))
    {
        let mut lhs = num_with_unit;
        for _ in 0..unit.1.len() {
            lhs.pop();
        }

        let (decimal_part, number_part) = modf(match lhs.parse::<f64>() {
            Ok(x) => x,
            Err(_) => return None,
        });

        let (num, unit_to_use) = match unit.2 {
            Some(unit_to_convert_to) => (
                Some(
                    ((number_part * unit_to_convert_to.1 as f64)
                        + (decimal_part * unit_to_convert_to.1 as f64)) as i64,
                ),
                unit_to_convert_to.0,
            ),
            None => (Some(number_part as i64), unit.0),
        };

        if let Some(x) = num {
            trace!("-- found {} {:?}", x, unit_to_use);

            let lhs_span = Span::new(span.start, span.start + lhs.len());
            let unit_span = Span::new(span.start + lhs.len(), span.end);
            return Some(Expression {
                expr: Expr::ValueWithUnit(
                    Box::new(Expression {
                        expr: Expr::Int(x),
                        span: lhs_span,
                        ty: Type::Number,
                        custom_completion: None,
                    }),
                    Spanned {
                        item: unit_to_use,
                        span: unit_span,
                    },
                ),
                span,
                ty: Type::Filesize,
                custom_completion: None,
            });
        }
    }

    None
}

pub fn parse_glob_pattern(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Expression, Option<ParseError>) {
    let bytes = working_set.get_span_contents(span);
    let (token, err) = unescape_unquote_string(bytes, span);
    trace!("parsing: glob pattern");

    if err.is_none() {
        trace!("-- found {}", token);
        (
            Expression {
                expr: Expr::GlobPattern(token),
                span,
                ty: Type::String,
                custom_completion: None,
            },
            None,
        )
    } else {
        (
            garbage(span),
            Some(ParseError::Expected("string".into(), span)),
        )
    }
}

pub fn unescape_string(bytes: &[u8], span: Span) -> (Vec<u8>, Option<ParseError>) {
    let mut output = Vec::new();

    let mut idx = 0;
    let mut err = None;

    while idx < bytes.len() {
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
                Some(b'u') => {
                    match (
                        bytes.get(idx + 1),
                        bytes.get(idx + 2),
                        bytes.get(idx + 3),
                        bytes.get(idx + 4),
                    ) {
                        (Some(h1), Some(h2), Some(h3), Some(h4)) => {
                            let s = String::from_utf8(vec![*h1, *h2, *h3, *h4]);

                            if let Ok(s) = s {
                                let int = u32::from_str_radix(&s, 16);

                                if let Ok(int) = int {
                                    let result = char::from_u32(int);

                                    if let Some(result) = result {
                                        let mut buffer = vec![0; 4];
                                        let result = result.encode_utf8(&mut buffer);

                                        for elem in result.bytes() {
                                            output.push(elem);
                                        }

                                        idx += 5;
                                        continue;
                                    }
                                }
                            }
                            err = Some(ParseError::Expected(
                                "unicode hex value".into(),
                                Span {
                                    start: (span.start + idx),
                                    end: span.end,
                                },
                            ));
                        }
                        _ => {
                            err = Some(ParseError::Expected(
                                "unicode hex value".into(),
                                Span {
                                    start: (span.start + idx),
                                    end: span.end,
                                },
                            ));
                        }
                    }
                    idx += 5;
                }
                _ => {
                    err = Some(ParseError::Expected(
                        "supported escape character".into(),
                        Span {
                            start: (span.start + idx),
                            end: span.end,
                        },
                    ));
                }
            }
        } else {
            output.push(bytes[idx]);
            idx += 1;
        }
    }

    (output, err)
}

pub fn unescape_unquote_string(bytes: &[u8], span: Span) -> (String, Option<ParseError>) {
    if bytes.starts_with(b"\"") {
        // Needs unescaping
        let bytes = trim_quotes(bytes);

        let (bytes, err) = unescape_string(bytes, span);

        if let Ok(token) = String::from_utf8(bytes) {
            (token, err)
        } else {
            (
                String::new(),
                Some(ParseError::Expected("string".into(), span)),
            )
        }
    } else {
        let bytes = trim_quotes(bytes);

        if let Ok(token) = String::from_utf8(bytes.into()) {
            (token, None)
        } else {
            (
                String::new(),
                Some(ParseError::Expected("string".into(), span)),
            )
        }
    }
}

pub fn parse_string(
    working_set: &mut StateWorkingSet,
    span: Span,
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    trace!("parsing: string");

    let bytes = working_set.get_span_contents(span);

    // Check for bare word interpolation
    if bytes[0] != b'\'' && bytes[0] != b'"' && bytes[0] != b'`' && bytes.contains(&b'(') {
        return parse_string_interpolation(working_set, span, expand_aliases_denylist);
    }

    let (s, err) = unescape_unquote_string(bytes, span);

    (
        Expression {
            expr: Expr::String(s),
            span,
            ty: Type::String,
            custom_completion: None,
        },
        err,
    )
}

pub fn parse_string_strict(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Expression, Option<ParseError>) {
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
            return (garbage(span), Some(ParseError::Unclosed("\"".into(), span)));
        }
        if bytes.starts_with(b"\'") && (bytes.len() == 1 || !bytes.ends_with(b"\'")) {
            return (garbage(span), Some(ParseError::Unclosed("\'".into(), span)));
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
        trace!("-- found {}", token);

        if quoted {
            (
                Expression {
                    expr: Expr::String(token),
                    span,
                    ty: Type::String,
                    custom_completion: None,
                },
                None,
            )
        } else if token.contains(' ') {
            (
                garbage(span),
                Some(ParseError::Expected("string".into(), span)),
            )
        } else {
            (
                Expression {
                    expr: Expr::String(token),
                    span,
                    ty: Type::String,
                    custom_completion: None,
                },
                None,
            )
        }
    } else {
        (
            garbage(span),
            Some(ParseError::Expected("string".into(), span)),
        )
    }
}

//TODO: Handle error case for unknown shapes
pub fn parse_shape_name(
    working_set: &StateWorkingSet,
    bytes: &[u8],
    span: Span,
) -> (SyntaxShape, Option<ParseError>) {
    let result = match bytes {
        b"any" => SyntaxShape::Any,
        b"binary" => SyntaxShape::Binary,
        b"block" => SyntaxShape::Block, //FIXME: Blocks should have known output types
        b"closure" => SyntaxShape::Closure(None), //FIXME: Blocks should have known output types
        b"cell-path" => SyntaxShape::CellPath,
        b"duration" => SyntaxShape::Duration,
        b"path" => SyntaxShape::Filepath,
        b"directory" => SyntaxShape::Directory,
        b"expr" => SyntaxShape::Expression,
        b"filesize" => SyntaxShape::Filesize,
        b"glob" => SyntaxShape::GlobPattern,
        b"int" => SyntaxShape::Int,
        b"math" => SyntaxShape::MathExpression,
        b"number" => SyntaxShape::Number,
        b"operator" => SyntaxShape::Operator,
        b"range" => SyntaxShape::Range,
        b"cond" => SyntaxShape::RowCondition,
        b"bool" => SyntaxShape::Boolean,
        b"signature" => SyntaxShape::Signature,
        b"string" => SyntaxShape::String,
        b"variable" => SyntaxShape::Variable,
        b"record" => SyntaxShape::Record,
        b"list" => SyntaxShape::List(Box::new(SyntaxShape::Any)),
        b"table" => SyntaxShape::Table,
        b"error" => SyntaxShape::Error,
        _ => {
            if bytes.contains(&b'@') {
                let str = String::from_utf8_lossy(bytes);
                let split: Vec<_> = str.split('@').collect();
                let (shape, err) = parse_shape_name(
                    working_set,
                    split[0].as_bytes(),
                    Span {
                        start: span.start,
                        end: span.start + split[0].len(),
                    },
                );
                let command_name = trim_quotes(split[1].as_bytes());

                let decl_id = working_set.find_decl(command_name, &Type::Any);

                if let Some(decl_id) = decl_id {
                    return (SyntaxShape::Custom(Box::new(shape), decl_id), err);
                } else {
                    return (
                        shape,
                        Some(ParseError::UnknownCommand(Span {
                            start: span.start + split[0].len() + 1,
                            end: span.end,
                        })),
                    );
                }
            } else {
                return (SyntaxShape::Any, Some(ParseError::UnknownType(span)));
            }
        }
    };

    (result, None)
}

pub fn parse_type(_working_set: &StateWorkingSet, bytes: &[u8]) -> Type {
    match bytes {
        b"int" => Type::Int,
        b"float" => Type::Float,
        b"range" => Type::Range,
        b"bool" => Type::Bool,
        b"string" => Type::String,
        b"block" => Type::Block,
        b"duration" => Type::Duration,
        b"date" => Type::Date,
        b"filesize" => Type::Filesize,
        b"number" => Type::Number,
        b"table" => Type::Table(vec![]), //FIXME
        b"error" => Type::Error,
        b"binary" => Type::Binary,

        _ => Type::Any,
    }
}

pub fn parse_import_pattern(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    let mut error = None;

    let (head, head_span) = if let Some(head_span) = spans.get(0) {
        (
            working_set.get_span_contents(*head_span).to_vec(),
            head_span,
        )
    } else {
        return (
            garbage(span(spans)),
            Some(ParseError::WrongImportPattern(span(spans))),
        );
    };

    let maybe_module_id = working_set.find_module(&head);

    let (import_pattern, err) = if let Some(tail_span) = spans.get(1) {
        // FIXME: expand this to handle deeper imports once we support module imports
        let tail = working_set.get_span_contents(*tail_span);
        if tail == b"*" {
            (
                ImportPattern {
                    head: ImportPatternHead {
                        name: head,
                        id: maybe_module_id,
                        span: *head_span,
                    },
                    members: vec![ImportPatternMember::Glob { span: *tail_span }],
                    hidden: HashSet::new(),
                },
                None,
            )
        } else if tail.starts_with(b"[") {
            let (result, err) = parse_list_expression(
                working_set,
                *tail_span,
                &SyntaxShape::String,
                expand_aliases_denylist,
            );
            error = error.or(err);

            let mut output = vec![];

            match result {
                Expression {
                    expr: Expr::List(list),
                    ..
                } => {
                    for expr in list {
                        let contents = working_set.get_span_contents(expr.span);
                        output.push((trim_quotes(contents).to_vec(), expr.span));
                    }

                    (
                        ImportPattern {
                            head: ImportPatternHead {
                                name: head,
                                id: maybe_module_id,
                                span: *head_span,
                            },
                            members: vec![ImportPatternMember::List { names: output }],
                            hidden: HashSet::new(),
                        },
                        None,
                    )
                }
                _ => (
                    ImportPattern {
                        head: ImportPatternHead {
                            name: head,
                            id: maybe_module_id,
                            span: *head_span,
                        },
                        members: vec![],
                        hidden: HashSet::new(),
                    },
                    Some(ParseError::ExportNotFound(result.span)),
                ),
            }
        } else {
            let tail = trim_quotes(tail);
            (
                ImportPattern {
                    head: ImportPatternHead {
                        name: head,
                        id: maybe_module_id,
                        span: *head_span,
                    },
                    members: vec![ImportPatternMember::Name {
                        name: tail.to_vec(),
                        span: *tail_span,
                    }],
                    hidden: HashSet::new(),
                },
                None,
            )
        }
    } else {
        (
            ImportPattern {
                head: ImportPatternHead {
                    name: head,
                    id: maybe_module_id,
                    span: *head_span,
                },
                members: vec![],
                hidden: HashSet::new(),
            },
            None,
        )
    };

    (
        Expression {
            expr: Expr::ImportPattern(import_pattern),
            span: span(&spans[1..]),
            ty: Type::List(Box::new(Type::String)),
            custom_completion: None,
        },
        error.or(err),
    )
}

pub fn parse_var_with_opt_type(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    spans_idx: &mut usize,
    mutable: bool,
) -> (Expression, Option<ParseError>) {
    let bytes = working_set.get_span_contents(spans[*spans_idx]).to_vec();

    if bytes.contains(&b' ')
        || bytes.contains(&b'"')
        || bytes.contains(&b'\'')
        || bytes.contains(&b'`')
    {
        return (
            garbage(spans[*spans_idx]),
            Some(ParseError::VariableNotValid(spans[*spans_idx])),
        );
    }

    if bytes.ends_with(b":") {
        // We end with colon, so the next span should be the type
        if *spans_idx + 1 < spans.len() {
            *spans_idx += 1;
            let type_bytes = working_set.get_span_contents(spans[*spans_idx]);

            let ty = parse_type(working_set, type_bytes);

            let var_name = bytes[0..(bytes.len() - 1)].to_vec();

            if !is_variable(&var_name) {
                return (
                    garbage(spans[*spans_idx]),
                    Some(ParseError::Expected(
                        "valid variable name".into(),
                        spans[*spans_idx],
                    )),
                );
            }

            let id = working_set.add_variable(var_name, spans[*spans_idx - 1], ty.clone(), mutable);

            (
                Expression {
                    expr: Expr::VarDecl(id),
                    span: span(&spans[*spans_idx - 1..*spans_idx + 1]),
                    ty,
                    custom_completion: None,
                },
                None,
            )
        } else {
            let var_name = bytes[0..(bytes.len() - 1)].to_vec();

            if !is_variable(&var_name) {
                return (
                    garbage(spans[*spans_idx]),
                    Some(ParseError::Expected(
                        "valid variable name".into(),
                        spans[*spans_idx],
                    )),
                );
            }

            let id = working_set.add_variable(var_name, spans[*spans_idx], Type::Any, mutable);
            (
                Expression {
                    expr: Expr::VarDecl(id),
                    span: spans[*spans_idx],
                    ty: Type::Any,
                    custom_completion: None,
                },
                Some(ParseError::MissingType(spans[*spans_idx])),
            )
        }
    } else {
        let var_name = bytes;

        if !is_variable(&var_name) {
            return (
                garbage(spans[*spans_idx]),
                Some(ParseError::Expected(
                    "valid variable name".into(),
                    spans[*spans_idx],
                )),
            );
        }

        let id = working_set.add_variable(
            var_name,
            span(&spans[*spans_idx..*spans_idx + 1]),
            Type::Any,
            mutable,
        );

        (
            Expression {
                expr: Expr::VarDecl(id),
                span: span(&spans[*spans_idx..*spans_idx + 1]),
                ty: Type::Any,
                custom_completion: None,
            },
            None,
        )
    }
}

pub fn expand_to_cell_path(
    working_set: &mut StateWorkingSet,
    expression: &mut Expression,
    var_id: VarId,
    expand_aliases_denylist: &[usize],
) {
    if let Expression {
        expr: Expr::String(_),
        span,
        ..
    } = expression
    {
        // Re-parse the string as if it were a cell-path
        let (new_expression, _err) =
            parse_full_cell_path(working_set, Some(var_id), *span, expand_aliases_denylist);

        *expression = new_expression;
    }
}

pub fn parse_row_condition(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    let var_id = working_set.add_variable(b"$it".to_vec(), span(spans), Type::Any, false);
    let (expression, err) =
        parse_math_expression(working_set, spans, Some(var_id), expand_aliases_denylist);
    let span = span(spans);

    let block_id = match expression.expr {
        Expr::Block(block_id) => block_id,
        Expr::Closure(block_id) => block_id,
        _ => {
            // We have an expression, so let's convert this into a block.
            let mut block = Block::new();
            let mut pipeline = Pipeline::new();
            pipeline.expressions.push(expression);

            block.pipelines.push(pipeline);

            block.signature.required_positional.push(PositionalArg {
                name: "$it".into(),
                desc: "row condition".into(),
                shape: SyntaxShape::Any,
                var_id: Some(var_id),
                default_value: None,
            });

            working_set.add_block(block)
        }
    };

    (
        Expression {
            ty: Type::Bool,
            span,
            expr: Expr::RowCondition(block_id),
            custom_completion: None,
        },
        err,
    )
}

pub fn parse_signature(
    working_set: &mut StateWorkingSet,
    span: Span,
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    let bytes = working_set.get_span_contents(span);

    let mut error = None;
    let mut start = span.start;
    let mut end = span.end;

    let mut has_paren = false;

    if bytes.starts_with(b"[") {
        start += 1;
    } else if bytes.starts_with(b"(") {
        has_paren = true;
        start += 1;
    } else {
        error = error.or_else(|| {
            Some(ParseError::Expected(
                "[ or (".into(),
                Span {
                    start,
                    end: start + 1,
                },
            ))
        });
    }

    if (has_paren && bytes.ends_with(b")")) || (!has_paren && bytes.ends_with(b"]")) {
        end -= 1;
    } else {
        error = error.or_else(|| {
            Some(ParseError::Unclosed(
                "] or )".into(),
                Span { start: end, end },
            ))
        });
    }

    let (sig, err) =
        parse_signature_helper(working_set, Span { start, end }, expand_aliases_denylist);
    error = error.or(err);

    (
        Expression {
            expr: Expr::Signature(sig),
            span,
            ty: Type::Signature,
            custom_completion: None,
        },
        error,
    )
}

pub fn parse_signature_helper(
    working_set: &mut StateWorkingSet,
    span: Span,
    expand_aliases_denylist: &[usize],
) -> (Box<Signature>, Option<ParseError>) {
    #[allow(clippy::enum_variant_names)]
    enum ParseMode {
        ArgMode,
        TypeMode,
        DefaultValueMode,
    }

    #[derive(Debug)]
    enum Arg {
        Positional(PositionalArg, bool), // bool - required
        RestPositional(PositionalArg),
        Flag(Flag),
    }

    let mut error = None;
    let source = working_set.get_span_contents(span);

    let (output, err) = lex(
        source,
        span.start,
        &[b'\n', b'\r', b','],
        &[b':', b'='],
        false,
    );
    error = error.or(err);

    let mut args: Vec<Arg> = vec![];
    let mut parse_mode = ParseMode::ArgMode;

    for token in &output {
        match token {
            Token {
                contents: crate::TokenContents::Item,
                span,
            } => {
                let span = *span;
                let contents = working_set.get_span_contents(span);

                if contents == b":" {
                    match parse_mode {
                        ParseMode::ArgMode => {
                            parse_mode = ParseMode::TypeMode;
                        }
                        ParseMode::TypeMode | ParseMode::DefaultValueMode => {
                            // We're seeing two types for the same thing for some reason, error
                            error =
                                error.or_else(|| Some(ParseError::Expected("type".into(), span)));
                        }
                    }
                } else if contents == b"=" {
                    match parse_mode {
                        ParseMode::ArgMode | ParseMode::TypeMode => {
                            parse_mode = ParseMode::DefaultValueMode;
                        }
                        ParseMode::DefaultValueMode => {
                            // We're seeing two default values for some reason, error
                            error = error.or_else(|| {
                                Some(ParseError::Expected("default value".into(), span))
                            });
                        }
                    }
                } else {
                    match parse_mode {
                        ParseMode::ArgMode => {
                            if contents.starts_with(b"--") && contents.len() > 2 {
                                // Long flag
                                let flags: Vec<_> =
                                    contents.split(|x| x == &b'(').map(|x| x.to_vec()).collect();

                                let long = String::from_utf8_lossy(&flags[0][2..]).to_string();
                                let mut variable_name = flags[0][2..].to_vec();
                                // Replace the '-' in a variable name with '_'
                                (0..variable_name.len()).for_each(|idx| {
                                    if variable_name[idx] == b'-' {
                                        variable_name[idx] = b'_';
                                    }
                                });

                                if !is_variable(&variable_name) {
                                    error = error.or_else(|| {
                                        Some(ParseError::Expected(
                                            "valid variable name".into(),
                                            span,
                                        ))
                                    })
                                }

                                let var_id =
                                    working_set.add_variable(variable_name, span, Type::Any, false);

                                if flags.len() == 1 {
                                    args.push(Arg::Flag(Flag {
                                        arg: None,
                                        desc: String::new(),
                                        long,
                                        short: None,
                                        required: false,
                                        var_id: Some(var_id),
                                        default_value: None,
                                    }));
                                } else if flags.len() >= 3 {
                                    error = error.or_else(|| {
                                        Some(ParseError::Expected("one short flag".into(), span))
                                    });
                                } else {
                                    let short_flag = &flags[1];
                                    let short_flag = if !short_flag.starts_with(b"-")
                                        || !short_flag.ends_with(b")")
                                    {
                                        error = error.or_else(|| {
                                            Some(ParseError::Expected("short flag".into(), span))
                                        });
                                        short_flag
                                    } else {
                                        &short_flag[1..(short_flag.len() - 1)]
                                    };

                                    let short_flag =
                                        String::from_utf8_lossy(short_flag).to_string();
                                    let chars: Vec<char> = short_flag.chars().collect();
                                    let long = String::from_utf8_lossy(&flags[0][2..]).to_string();
                                    let mut variable_name = flags[0][2..].to_vec();

                                    (0..variable_name.len()).for_each(|idx| {
                                        if variable_name[idx] == b'-' {
                                            variable_name[idx] = b'_';
                                        }
                                    });

                                    if !is_variable(&variable_name) {
                                        error = error.or_else(|| {
                                            Some(ParseError::Expected(
                                                "valid variable name".into(),
                                                span,
                                            ))
                                        })
                                    }

                                    let var_id = working_set.add_variable(
                                        variable_name,
                                        span,
                                        Type::Any,
                                        false,
                                    );

                                    if chars.len() == 1 {
                                        args.push(Arg::Flag(Flag {
                                            arg: None,
                                            desc: String::new(),
                                            long,
                                            short: Some(chars[0]),
                                            required: false,
                                            var_id: Some(var_id),
                                            default_value: None,
                                        }));
                                    } else {
                                        error = error.or_else(|| {
                                            Some(ParseError::Expected("short flag".into(), span))
                                        });
                                    }
                                }
                            } else if contents.starts_with(b"-") && contents.len() > 1 {
                                // Short flag

                                let short_flag = &contents[1..];
                                let short_flag = String::from_utf8_lossy(short_flag).to_string();
                                let chars: Vec<char> = short_flag.chars().collect();

                                if chars.len() > 1 {
                                    error = error.or_else(|| {
                                        Some(ParseError::Expected("short flag".into(), span))
                                    });
                                }

                                let mut encoded_var_name = vec![0u8; 4];
                                let len = chars[0].encode_utf8(&mut encoded_var_name).len();
                                let variable_name = encoded_var_name[0..len].to_vec();
                                if !is_variable(&variable_name) {
                                    error = error.or_else(|| {
                                        Some(ParseError::Expected(
                                            "valid variable name".into(),
                                            span,
                                        ))
                                    })
                                }

                                let var_id =
                                    working_set.add_variable(variable_name, span, Type::Any, false);

                                args.push(Arg::Flag(Flag {
                                    arg: None,
                                    desc: String::new(),
                                    long: String::new(),
                                    short: Some(chars[0]),
                                    required: false,
                                    var_id: Some(var_id),
                                    default_value: None,
                                }));
                            } else if contents.starts_with(b"(-") {
                                let short_flag = &contents[2..];

                                let short_flag = if !short_flag.ends_with(b")") {
                                    error = error.or_else(|| {
                                        Some(ParseError::Expected("short flag".into(), span))
                                    });
                                    short_flag
                                } else {
                                    &short_flag[..(short_flag.len() - 1)]
                                };

                                let short_flag = String::from_utf8_lossy(short_flag).to_string();
                                let chars: Vec<char> = short_flag.chars().collect();

                                if chars.len() == 1 {
                                    match args.last_mut() {
                                        Some(Arg::Flag(flag)) => {
                                            if flag.short.is_some() {
                                                error = error.or_else(|| {
                                                    Some(ParseError::Expected(
                                                        "one short flag".into(),
                                                        span,
                                                    ))
                                                });
                                            } else {
                                                flag.short = Some(chars[0]);
                                            }
                                        }
                                        _ => {
                                            error = error.or_else(|| {
                                                Some(ParseError::Expected(
                                                    "unknown flag".into(),
                                                    span,
                                                ))
                                            });
                                        }
                                    }
                                } else {
                                    error = error.or_else(|| {
                                        Some(ParseError::Expected("short flag".into(), span))
                                    });
                                }
                            } else if contents.ends_with(b"?") {
                                let contents: Vec<_> = contents[..(contents.len() - 1)].into();
                                let name = String::from_utf8_lossy(&contents).to_string();

                                if !is_variable(&contents) {
                                    error = error.or_else(|| {
                                        Some(ParseError::Expected(
                                            "valid variable name".into(),
                                            span,
                                        ))
                                    })
                                }

                                let var_id =
                                    working_set.add_variable(contents, span, Type::Any, false);

                                // Positional arg, optional
                                args.push(Arg::Positional(
                                    PositionalArg {
                                        desc: String::new(),
                                        name,
                                        shape: SyntaxShape::Any,
                                        var_id: Some(var_id),
                                        default_value: None,
                                    },
                                    false,
                                ))
                            } else if let Some(contents) = contents.strip_prefix(b"...") {
                                let name = String::from_utf8_lossy(contents).to_string();
                                let contents_vec: Vec<u8> = contents.to_vec();
                                if !is_variable(&contents_vec) {
                                    error = error.or_else(|| {
                                        Some(ParseError::Expected(
                                            "valid variable name".into(),
                                            span,
                                        ))
                                    })
                                }

                                let var_id =
                                    working_set.add_variable(contents_vec, span, Type::Any, false);

                                args.push(Arg::RestPositional(PositionalArg {
                                    desc: String::new(),
                                    name,
                                    shape: SyntaxShape::Any,
                                    var_id: Some(var_id),
                                    default_value: None,
                                }));
                            } else {
                                let name = String::from_utf8_lossy(contents).to_string();
                                let contents_vec = contents.to_vec();

                                if !is_variable(&contents_vec) {
                                    error = error.or_else(|| {
                                        Some(ParseError::Expected(
                                            "valid variable name".into(),
                                            span,
                                        ))
                                    })
                                }

                                let var_id =
                                    working_set.add_variable(contents_vec, span, Type::Any, false);

                                // Positional arg, required
                                args.push(Arg::Positional(
                                    PositionalArg {
                                        desc: String::new(),
                                        name,
                                        shape: SyntaxShape::Any,
                                        var_id: Some(var_id),
                                        default_value: None,
                                    },
                                    true,
                                ))
                            }
                        }
                        ParseMode::TypeMode => {
                            if let Some(last) = args.last_mut() {
                                let (syntax_shape, err) =
                                    parse_shape_name(working_set, contents, span);
                                error = error.or(err);
                                //TODO check if we're replacing a custom parameter already
                                match last {
                                    Arg::Positional(PositionalArg { shape, var_id, .. }, ..) => {
                                        working_set.set_variable_type(var_id.expect("internal error: all custom parameters must have var_ids"), syntax_shape.to_type());
                                        *shape = syntax_shape;
                                    }
                                    Arg::RestPositional(PositionalArg {
                                        shape, var_id, ..
                                    }) => {
                                        working_set.set_variable_type(var_id.expect("internal error: all custom parameters must have var_ids"), syntax_shape.to_type());
                                        *shape = syntax_shape;
                                    }
                                    Arg::Flag(Flag { arg, var_id, .. }) => {
                                        // Flags with a boolean type are just present/not-present switches
                                        if syntax_shape != SyntaxShape::Boolean {
                                            working_set.set_variable_type(var_id.expect("internal error: all custom parameters must have var_ids"), syntax_shape.to_type());
                                            *arg = Some(syntax_shape)
                                        }
                                    }
                                }
                            }
                            parse_mode = ParseMode::ArgMode;
                        }
                        ParseMode::DefaultValueMode => {
                            if let Some(last) = args.last_mut() {
                                let (expression, err) = parse_value(
                                    working_set,
                                    span,
                                    &SyntaxShape::Any,
                                    expand_aliases_denylist,
                                );
                                error = error.or(err);

                                //TODO check if we're replacing a custom parameter already
                                match last {
                                    Arg::Positional(
                                        PositionalArg {
                                            shape,
                                            var_id,
                                            default_value,
                                            ..
                                        },
                                        required,
                                    ) => {
                                        let var_id = var_id.expect("internal error: all custom parameters must have var_ids");
                                        let var_type = &working_set.get_variable(var_id).ty;
                                        match var_type {
                                            Type::Any => {
                                                working_set.set_variable_type(
                                                    var_id,
                                                    expression.ty.clone(),
                                                );
                                            }
                                            t => {
                                                if t != &expression.ty {
                                                    error = error.or_else(|| {
                                                        Some(ParseError::AssignmentMismatch(
                                                            "Default value wrong type".into(),
                                                            format!("default value not {}", t),
                                                            expression.span,
                                                        ))
                                                    })
                                                }
                                            }
                                        }
                                        *shape = expression.ty.to_shape();
                                        *default_value = Some(expression);
                                        *required = false;
                                    }
                                    Arg::RestPositional(..) => {
                                        error = error.or_else(|| {
                                            Some(ParseError::AssignmentMismatch(
                                                "Rest parameter given default value".into(),
                                                "can't have default value".into(),
                                                expression.span,
                                            ))
                                        })
                                    }
                                    Arg::Flag(Flag {
                                        arg,
                                        var_id,
                                        default_value,
                                        ..
                                    }) => {
                                        let var_id = var_id.expect("internal error: all custom parameters must have var_ids");
                                        let var_type = &working_set.get_variable(var_id).ty;

                                        let expression_ty = expression.ty.clone();
                                        let expression_span = expression.span;

                                        *default_value = Some(expression);

                                        // Flags with a boolean type are just present/not-present switches
                                        if var_type != &Type::Bool {
                                            match var_type {
                                                Type::Any => {
                                                    *arg = Some(expression_ty.to_shape());
                                                    working_set
                                                        .set_variable_type(var_id, expression_ty);
                                                }
                                                t => {
                                                    if t != &expression_ty {
                                                        error = error.or_else(|| {
                                                            Some(ParseError::AssignmentMismatch(
                                                                "Default value wrong type".into(),
                                                                format!("default value not {}", t),
                                                                expression_span,
                                                            ))
                                                        })
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            parse_mode = ParseMode::ArgMode;
                        }
                    }
                }
            }
            Token {
                contents: crate::TokenContents::Comment,
                span,
            } => {
                let contents = working_set.get_span_contents(Span {
                    start: span.start + 1,
                    end: span.end,
                });

                let mut contents = String::from_utf8_lossy(contents).to_string();
                contents = contents.trim().into();

                if let Some(last) = args.last_mut() {
                    match last {
                        Arg::Flag(flag) => {
                            if !flag.desc.is_empty() {
                                flag.desc.push('\n');
                            }
                            flag.desc.push_str(&contents);
                        }
                        Arg::Positional(positional, ..) => {
                            if !positional.desc.is_empty() {
                                positional.desc.push('\n');
                            }
                            positional.desc.push_str(&contents);
                        }
                        Arg::RestPositional(positional) => {
                            if !positional.desc.is_empty() {
                                positional.desc.push('\n');
                            }
                            positional.desc.push_str(&contents);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let mut sig = Signature::new(String::new());

    for arg in args {
        match arg {
            Arg::Positional(positional, required) => {
                if required {
                    if !sig.optional_positional.is_empty() {
                        error = error.or_else(|| {
                            Some(ParseError::RequiredAfterOptional(
                                positional.name.clone(),
                                span,
                            ))
                        })
                    }
                    sig.required_positional.push(positional)
                } else {
                    sig.optional_positional.push(positional)
                }
            }
            Arg::Flag(flag) => sig.named.push(flag),
            Arg::RestPositional(positional) => {
                if positional.name.is_empty() {
                    error = error.or(Some(ParseError::RestNeedsName(span)))
                } else if sig.rest_positional.is_none() {
                    sig.rest_positional = Some(PositionalArg {
                        name: positional.name,
                        ..positional
                    })
                } else {
                    // Too many rest params
                    error = error.or(Some(ParseError::MultipleRestParams(span)))
                }
            }
        }
    }

    (Box::new(sig), error)
}

pub fn parse_list_expression(
    working_set: &mut StateWorkingSet,
    span: Span,
    element_shape: &SyntaxShape,
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    let bytes = working_set.get_span_contents(span);

    let mut error = None;

    let mut start = span.start;
    let mut end = span.end;

    if bytes.starts_with(b"[") {
        start += 1;
    }
    if bytes.ends_with(b"]") {
        end -= 1;
    } else {
        error = error.or_else(|| Some(ParseError::Unclosed("]".into(), Span { start: end, end })));
    }

    let inner_span = Span { start, end };
    let source = working_set.get_span_contents(inner_span);

    let (output, err) = lex(source, inner_span.start, &[b'\n', b'\r', b','], &[], true);
    error = error.or(err);

    let (output, err) = lite_parse(&output);
    error = error.or(err);

    let mut args = vec![];

    let mut contained_type: Option<Type> = None;

    if !output.block.is_empty() {
        for arg in &output.block[0].commands {
            let mut spans_idx = 0;

            while spans_idx < arg.parts.len() {
                let (arg, err) = parse_multispan_value(
                    working_set,
                    &arg.parts,
                    &mut spans_idx,
                    element_shape,
                    expand_aliases_denylist,
                );
                error = error.or(err);

                if let Some(ref ctype) = contained_type {
                    if *ctype != arg.ty {
                        contained_type = Some(Type::Any);
                    }
                } else {
                    contained_type = Some(arg.ty.clone());
                }

                args.push(arg);

                spans_idx += 1;
            }
        }
    }

    (
        Expression {
            expr: Expr::List(args),
            span,
            ty: Type::List(Box::new(if let Some(ty) = contained_type {
                ty
            } else {
                Type::Any
            })),
            custom_completion: None,
        },
        error,
    )
}

pub fn parse_table_expression(
    working_set: &mut StateWorkingSet,
    original_span: Span,
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    let bytes = working_set.get_span_contents(original_span);
    let mut error = None;

    let mut start = original_span.start;
    let mut end = original_span.end;

    if bytes.starts_with(b"[") {
        start += 1;
    }
    if bytes.ends_with(b"]") {
        end -= 1;
    } else {
        error = error.or_else(|| Some(ParseError::Unclosed("]".into(), Span { start: end, end })));
    }

    let inner_span = Span { start, end };

    let source = working_set.get_span_contents(inner_span);

    let (output, err) = lex(source, start, &[b'\n', b'\r', b','], &[], true);
    error = error.or(err);

    let (output, err) = lite_parse(&output);
    error = error.or(err);

    match output.block.len() {
        0 => (
            Expression {
                expr: Expr::List(vec![]),
                span: original_span,
                ty: Type::List(Box::new(Type::Any)),
                custom_completion: None,
            },
            None,
        ),
        1 => {
            // List
            parse_list_expression(
                working_set,
                original_span,
                &SyntaxShape::Any,
                expand_aliases_denylist,
            )
        }
        _ => {
            let mut table_headers = vec![];

            let (headers, err) = parse_value(
                working_set,
                output.block[0].commands[0].parts[0],
                &SyntaxShape::List(Box::new(SyntaxShape::Any)),
                expand_aliases_denylist,
            );
            error = error.or(err);

            if let Expression {
                expr: Expr::List(headers),
                ..
            } = headers
            {
                table_headers = headers;
            }

            let mut rows = vec![];
            for part in &output.block[1].commands[0].parts {
                let (values, err) = parse_value(
                    working_set,
                    *part,
                    &SyntaxShape::List(Box::new(SyntaxShape::Any)),
                    expand_aliases_denylist,
                );
                error = error.or(err);
                if let Expression {
                    expr: Expr::List(values),
                    span,
                    ..
                } = values
                {
                    match values.len().cmp(&table_headers.len()) {
                        std::cmp::Ordering::Less => {
                            error = error
                                .or(Some(ParseError::MissingColumns(table_headers.len(), span)))
                        }
                        std::cmp::Ordering::Equal => {}
                        std::cmp::Ordering::Greater => {
                            error = error.or_else(|| {
                                Some(ParseError::ExtraColumns(
                                    table_headers.len(),
                                    values[table_headers.len()].span,
                                ))
                            })
                        }
                    }

                    rows.push(values);
                }
            }

            (
                Expression {
                    expr: Expr::Table(table_headers, rows),
                    span: original_span,
                    ty: Type::Table(vec![]), //FIXME
                    custom_completion: None,
                },
                error,
            )
        }
    }
}

pub fn parse_block_expression(
    working_set: &mut StateWorkingSet,
    shape: &SyntaxShape,
    span: Span,
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    trace!("parsing: block expression");

    let bytes = working_set.get_span_contents(span);
    let mut error = None;

    let mut start = span.start;
    let mut end = span.end;

    if bytes.starts_with(b"{") {
        start += 1;
    } else {
        return (
            garbage(span),
            Some(ParseError::Expected("block".into(), span)),
        );
    }
    if bytes.ends_with(b"}") {
        end -= 1;
    } else {
        error = error.or_else(|| Some(ParseError::Unclosed("}".into(), Span { start: end, end })));
    }

    let inner_span = Span { start, end };

    let source = working_set.get_span_contents(inner_span);

    let (output, err) = lex(source, start, &[], &[], false);
    error = error.or(err);

    working_set.enter_scope();

    // Check to see if we have parameters
    let (signature, amt_to_skip): (Option<(Box<Signature>, Span)>, usize) = match output.first() {
        Some(Token {
            contents: TokenContents::Pipe,
            span,
        }) => {
            error = error.or_else(|| {
                Some(ParseError::Expected(
                    "block but found closure".into(),
                    *span,
                ))
            });
            (None, 0)
        }
        _ => (None, 0),
    };

    let (output, err) = lite_parse(&output[amt_to_skip..]);
    error = error.or(err);

    // TODO: Finish this
    if let SyntaxShape::Closure(Some(v)) = shape {
        if let Some((sig, sig_span)) = &signature {
            if sig.num_positionals() > v.len() {
                error = error.or_else(|| {
                    Some(ParseError::Expected(
                        format!(
                            "{} block parameter{}",
                            v.len(),
                            if v.len() > 1 { "s" } else { "" }
                        ),
                        *sig_span,
                    ))
                });
            }

            for (expected, PositionalArg { name, shape, .. }) in
                v.iter().zip(sig.required_positional.iter())
            {
                if expected != shape && *shape != SyntaxShape::Any {
                    error = error.or_else(|| {
                        Some(ParseError::ParameterMismatchType(
                            name.to_owned(),
                            expected.to_string(),
                            shape.to_string(),
                            *sig_span,
                        ))
                    });
                }
            }
        }
    }

    let (mut output, err) =
        parse_block(working_set, &output, false, expand_aliases_denylist, false);
    error = error.or(err);

    if let Some(signature) = signature {
        output.signature = signature.0;
    } else if let Some(last) = working_set.delta.scope.last() {
        // FIXME: this only supports the top $it. Is this sufficient?

        if let Some(var_id) = last.get_var(b"$it") {
            let mut signature = Signature::new("");
            signature.required_positional.push(PositionalArg {
                var_id: Some(*var_id),
                name: "$it".into(),
                desc: String::new(),
                shape: SyntaxShape::Any,
                default_value: None,
            });
            output.signature = Box::new(signature);
        }
    }

    output.span = Some(span);

    working_set.exit_scope();

    let block_id = working_set.add_block(output);

    (
        Expression {
            expr: Expr::Block(block_id),
            span,
            ty: Type::Block,
            custom_completion: None,
        },
        error,
    )
}

pub fn parse_closure_expression(
    working_set: &mut StateWorkingSet,
    shape: &SyntaxShape,
    span: Span,
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    trace!("parsing: closure expression");

    let bytes = working_set.get_span_contents(span);
    let mut error = None;

    let mut start = span.start;
    let mut end = span.end;

    if bytes.starts_with(b"{") {
        start += 1;
    } else {
        return (
            garbage(span),
            Some(ParseError::Expected("block".into(), span)),
        );
    }
    if bytes.ends_with(b"}") {
        end -= 1;
    } else {
        error = error.or_else(|| Some(ParseError::Unclosed("}".into(), Span { start: end, end })));
    }

    let inner_span = Span { start, end };

    let source = working_set.get_span_contents(inner_span);

    let (output, err) = lex(source, start, &[], &[], false);
    error = error.or(err);

    working_set.enter_scope();

    // Check to see if we have parameters
    let (signature, amt_to_skip): (Option<(Box<Signature>, Span)>, usize) = match output.first() {
        Some(Token {
            contents: TokenContents::Pipe,
            span,
        }) => {
            // We've found a parameter list
            let start_point = span.start;
            let mut token_iter = output.iter().enumerate().skip(1);
            let mut end_span = None;
            let mut amt_to_skip = 1;

            for token in &mut token_iter {
                if let Token {
                    contents: TokenContents::Pipe,
                    span,
                } = token.1
                {
                    end_span = Some(span);
                    amt_to_skip = token.0;
                    break;
                }
            }

            let end_point = if let Some(span) = end_span {
                span.end
            } else {
                end
            };

            let signature_span = Span {
                start: start_point,
                end: end_point,
            };
            let (signature, err) =
                parse_signature_helper(working_set, signature_span, expand_aliases_denylist);
            error = error.or(err);

            (Some((signature, signature_span)), amt_to_skip)
        }
        Some(Token {
            contents: TokenContents::Item,
            span,
        }) => {
            let contents = working_set.get_span_contents(*span);
            if contents == b"||" {
                (
                    Some((Box::new(Signature::new("closure".to_string())), *span)),
                    1,
                )
            } else {
                (None, 0)
            }
        }
        _ => (None, 0),
    };

    let (output, err) = lite_parse(&output[amt_to_skip..]);
    error = error.or(err);

    // TODO: Finish this
    if let SyntaxShape::Closure(Some(v)) = shape {
        if let Some((sig, sig_span)) = &signature {
            if sig.num_positionals() > v.len() {
                error = error.or_else(|| {
                    Some(ParseError::Expected(
                        format!(
                            "{} block parameter{}",
                            v.len(),
                            if v.len() > 1 { "s" } else { "" }
                        ),
                        *sig_span,
                    ))
                });
            }

            for (expected, PositionalArg { name, shape, .. }) in
                v.iter().zip(sig.required_positional.iter())
            {
                if expected != shape && *shape != SyntaxShape::Any {
                    error = error.or_else(|| {
                        Some(ParseError::ParameterMismatchType(
                            name.to_owned(),
                            expected.to_string(),
                            shape.to_string(),
                            *sig_span,
                        ))
                    });
                }
            }
        }
    }

    let (mut output, err) =
        parse_block(working_set, &output, false, expand_aliases_denylist, false);
    error = error.or(err);

    if let Some(signature) = signature {
        output.signature = signature.0;
    } else if let Some(last) = working_set.delta.scope.last() {
        // FIXME: this only supports the top $it. Is this sufficient?

        if let Some(var_id) = last.get_var(b"$it") {
            let mut signature = Signature::new("");
            signature.required_positional.push(PositionalArg {
                var_id: Some(*var_id),
                name: "$it".into(),
                desc: String::new(),
                shape: SyntaxShape::Any,
                default_value: None,
            });
            output.signature = Box::new(signature);
        }
    }

    output.span = Some(span);

    working_set.exit_scope();

    let block_id = working_set.add_block(output);

    (
        Expression {
            expr: Expr::Closure(block_id),
            span,
            ty: Type::Closure,
            custom_completion: None,
        },
        error,
    )
}

pub fn parse_value(
    working_set: &mut StateWorkingSet,
    span: Span,
    shape: &SyntaxShape,
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    let bytes = working_set.get_span_contents(span);

    if bytes.is_empty() {
        return (garbage(span), Some(ParseError::IncompleteParser(span)));
    }

    // First, check the special-cases. These will likely represent specific values as expressions
    // and may fit a variety of shapes.
    //
    // We check variable first because immediately following we check for variables with cell paths
    // which might result in a value that fits other shapes (and require the variable to already be
    // declared)
    if shape == &SyntaxShape::Variable {
        trace!("parsing: variable");

        return parse_variable_expr(working_set, span);
    }

    // Check for reserved keyword values
    match bytes {
        b"true" => {
            if matches!(shape, SyntaxShape::Boolean) || matches!(shape, SyntaxShape::Any) {
                return (
                    Expression {
                        expr: Expr::Bool(true),
                        span,
                        ty: Type::Bool,
                        custom_completion: None,
                    },
                    None,
                );
            } else {
                return (
                    Expression::garbage(span),
                    Some(ParseError::Expected("non-boolean value".into(), span)),
                );
            }
        }
        b"false" => {
            if matches!(shape, SyntaxShape::Boolean) || matches!(shape, SyntaxShape::Any) {
                return (
                    Expression {
                        expr: Expr::Bool(false),
                        span,
                        ty: Type::Bool,
                        custom_completion: None,
                    },
                    None,
                );
            } else {
                return (
                    Expression::garbage(span),
                    Some(ParseError::Expected("non-boolean value".into(), span)),
                );
            }
        }
        b"null" => {
            return (
                Expression {
                    expr: Expr::Nothing,
                    span,
                    ty: Type::Nothing,
                    custom_completion: None,
                },
                None,
            );
        }

        _ => {}
    }

    match bytes[0] {
        b'$' => return parse_dollar_expr(working_set, span, expand_aliases_denylist),
        b'(' => {
            if let (expr, None) = parse_range(working_set, span, expand_aliases_denylist) {
                return (expr, None);
            } else if matches!(shape, SyntaxShape::Signature) {
                return parse_signature(working_set, span, expand_aliases_denylist);
            } else {
                return parse_full_cell_path(working_set, None, span, expand_aliases_denylist);
            }
        }
        b'{' => {
            if !matches!(shape, SyntaxShape::Closure(..)) && !matches!(shape, SyntaxShape::Block) {
                if let (expr, None) =
                    parse_full_cell_path(working_set, None, span, expand_aliases_denylist)
                {
                    return (expr, None);
                }
            }
            if matches!(shape, SyntaxShape::Closure(_)) || matches!(shape, SyntaxShape::Any) {
                return parse_closure_expression(working_set, shape, span, expand_aliases_denylist);
            } else if matches!(shape, SyntaxShape::Block) {
                return parse_block_expression(working_set, shape, span, expand_aliases_denylist);
            } else if matches!(shape, SyntaxShape::Record) {
                return parse_record(working_set, span, expand_aliases_denylist);
            } else {
                return (
                    Expression::garbage(span),
                    Some(ParseError::Expected("non-block value".into(), span)),
                );
            }
        }
        b'[' => match shape {
            SyntaxShape::Any
            | SyntaxShape::List(_)
            | SyntaxShape::Table
            | SyntaxShape::Signature => {}
            _ => {
                return (
                    Expression::garbage(span),
                    Some(ParseError::Expected("non-[] value".into(), span)),
                );
            }
        },
        _ => {}
    }

    match shape {
        SyntaxShape::Custom(shape, custom_completion) => {
            let (mut expression, err) =
                parse_value(working_set, span, shape, expand_aliases_denylist);
            expression.custom_completion = Some(*custom_completion);
            (expression, err)
        }
        SyntaxShape::Number => parse_number(bytes, span),
        SyntaxShape::Int => parse_int(bytes, span),
        SyntaxShape::Duration => parse_duration(working_set, span),
        SyntaxShape::DateTime => parse_datetime(working_set, span),
        SyntaxShape::Filesize => parse_filesize(working_set, span),
        SyntaxShape::Range => parse_range(working_set, span, expand_aliases_denylist),
        SyntaxShape::Filepath => parse_filepath(working_set, span),
        SyntaxShape::Directory => parse_directory(working_set, span),
        SyntaxShape::GlobPattern => parse_glob_pattern(working_set, span),
        SyntaxShape::String => parse_string(working_set, span, expand_aliases_denylist),
        SyntaxShape::Binary => parse_binary(working_set, span),
        SyntaxShape::Signature => {
            if bytes.starts_with(b"[") {
                parse_signature(working_set, span, expand_aliases_denylist)
            } else {
                (
                    Expression::garbage(span),
                    Some(ParseError::Expected("signature".into(), span)),
                )
            }
        }
        SyntaxShape::List(elem) => {
            if bytes.starts_with(b"[") {
                parse_list_expression(working_set, span, elem, expand_aliases_denylist)
            } else {
                (
                    Expression::garbage(span),
                    Some(ParseError::Expected("list".into(), span)),
                )
            }
        }
        SyntaxShape::Table => {
            if bytes.starts_with(b"[") {
                parse_table_expression(working_set, span, expand_aliases_denylist)
            } else {
                (
                    Expression::garbage(span),
                    Some(ParseError::Expected("table".into(), span)),
                )
            }
        }
        SyntaxShape::CellPath => {
            let source = working_set.get_span_contents(span);
            let mut error = None;

            let (tokens, err) = lex(source, span.start, &[b'\n', b'\r'], &[b'.'], true);
            error = error.or(err);

            let tokens = tokens.into_iter().peekable();

            let (cell_path, err) =
                parse_cell_path(working_set, tokens, false, expand_aliases_denylist, span);
            error = error.or(err);

            (
                Expression {
                    expr: Expr::CellPath(CellPath { members: cell_path }),
                    span,
                    ty: Type::CellPath,
                    custom_completion: None,
                },
                error,
            )
        }
        SyntaxShape::Boolean => {
            // Redundant, though we catch bad boolean parses here
            if bytes == b"true" || bytes == b"false" {
                (
                    Expression {
                        expr: Expr::Bool(true),
                        span,
                        ty: Type::Bool,
                        custom_completion: None,
                    },
                    None,
                )
            } else {
                (
                    garbage(span),
                    Some(ParseError::Expected("bool".into(), span)),
                )
            }
        }
        SyntaxShape::Any => {
            if bytes.starts_with(b"[") {
                //parse_value(working_set, span, &SyntaxShape::Table)
                parse_full_cell_path(working_set, None, span, expand_aliases_denylist)
            } else {
                let shapes = [
                    SyntaxShape::Binary,
                    SyntaxShape::Int,
                    SyntaxShape::Number,
                    SyntaxShape::Range,
                    SyntaxShape::DateTime,
                    SyntaxShape::Filesize,
                    SyntaxShape::Duration,
                    SyntaxShape::Record,
                    SyntaxShape::Closure(None),
                    SyntaxShape::Block,
                    SyntaxShape::String,
                ];
                for shape in shapes.iter() {
                    if let (s, None) =
                        parse_value(working_set, span, shape, expand_aliases_denylist)
                    {
                        return (s, None);
                    }
                }
                (
                    garbage(span),
                    Some(ParseError::Expected("any shape".into(), span)),
                )
            }
        }
        _ => (garbage(span), Some(ParseError::IncompleteParser(span))),
    }
}

pub fn parse_operator(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Expression, Option<ParseError>) {
    let contents = working_set.get_span_contents(span);

    let operator = match contents {
        b"=" => Operator::Assignment(Assignment::Assign),
        b"+=" => Operator::Assignment(Assignment::PlusAssign),
        b"-=" => Operator::Assignment(Assignment::MinusAssign),
        b"*=" => Operator::Assignment(Assignment::MultiplyAssign),
        b"/=" => Operator::Assignment(Assignment::DivideAssign),
        b"==" => Operator::Comparison(Comparison::Equal),
        b"!=" => Operator::Comparison(Comparison::NotEqual),
        b"<" => Operator::Comparison(Comparison::LessThan),
        b"<=" => Operator::Comparison(Comparison::LessThanOrEqual),
        b">" => Operator::Comparison(Comparison::GreaterThan),
        b">=" => Operator::Comparison(Comparison::GreaterThanOrEqual),
        b"=~" => Operator::Comparison(Comparison::RegexMatch),
        b"!~" => Operator::Comparison(Comparison::NotRegexMatch),
        b"+" => Operator::Math(Math::Plus),
        b"++" => Operator::Math(Math::Append),
        b"-" => Operator::Math(Math::Minus),
        b"*" => Operator::Math(Math::Multiply),
        b"/" => Operator::Math(Math::Divide),
        b"//" => Operator::Math(Math::FloorDivision),
        b"in" => Operator::Comparison(Comparison::In),
        b"not-in" => Operator::Comparison(Comparison::NotIn),
        b"mod" => Operator::Math(Math::Modulo),
        b"bit-or" => Operator::Bits(Bits::BitOr),
        b"bit-xor" => Operator::Bits(Bits::BitXor),
        b"bit-and" => Operator::Bits(Bits::BitAnd),
        b"bit-shl" => Operator::Bits(Bits::ShiftLeft),
        b"bit-shr" => Operator::Bits(Bits::ShiftRight),
        b"starts-with" => Operator::Comparison(Comparison::StartsWith),
        b"ends-with" => Operator::Comparison(Comparison::EndsWith),
        b"&&" | b"and" => Operator::Boolean(Boolean::And),
        b"||" | b"or" => Operator::Boolean(Boolean::Or),
        b"**" => Operator::Math(Math::Pow),
        _ => {
            return (
                garbage(span),
                Some(ParseError::Expected("operator".into(), span)),
            );
        }
    };

    (
        Expression {
            expr: Expr::Operator(operator),
            span,
            ty: Type::Any,
            custom_completion: None,
        },
        None,
    )
}

pub fn parse_math_expression(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    lhs_row_var_id: Option<VarId>,
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    // As the expr_stack grows, we increase the required precedence to grow larger
    // If, at any time, the operator we're looking at is the same or lower precedence
    // of what is in the expression stack, we collapse the expression stack.
    //
    // This leads to an expression stack that grows under increasing precedence and collapses
    // under decreasing/sustained precedence
    //
    // The end result is a stack that we can fold into binary operations as right associations
    // safely.

    let mut expr_stack: Vec<Expression> = vec![];

    let mut idx = 0;
    let mut last_prec = 1000000;

    let mut error = None;

    let first_span = working_set.get_span_contents(spans[0]);

    if first_span == b"not" {
        if spans.len() > 1 {
            let (remainder, err) = parse_math_expression(
                working_set,
                &spans[1..],
                lhs_row_var_id,
                expand_aliases_denylist,
            );
            return (
                Expression {
                    expr: Expr::UnaryNot(Box::new(remainder)),
                    span: span(spans),
                    ty: Type::Bool,
                    custom_completion: None,
                },
                err,
            );
        } else {
            return (
                garbage(spans[0]),
                Some(ParseError::Expected(
                    "expression".into(),
                    Span {
                        start: spans[0].end,
                        end: spans[0].end,
                    },
                )),
            );
        }
    }

    let (mut lhs, err) = parse_value(
        working_set,
        spans[0],
        &SyntaxShape::Any,
        expand_aliases_denylist,
    );
    error = error.or(err);
    idx += 1;

    if idx >= spans.len() {
        // We already found the one part of our expression, so let's expand
        if let Some(row_var_id) = lhs_row_var_id {
            expand_to_cell_path(working_set, &mut lhs, row_var_id, expand_aliases_denylist);
        }
    }

    expr_stack.push(lhs);

    while idx < spans.len() {
        let (op, err) = parse_operator(working_set, spans[idx]);
        error = error.or(err);

        let op_prec = op.precedence();

        idx += 1;

        if idx == spans.len() {
            // Handle broken math expr `1 +` etc
            error = error.or(Some(ParseError::IncompleteMathExpression(spans[idx - 1])));

            expr_stack.push(Expression::garbage(spans[idx - 1]));
            expr_stack.push(Expression::garbage(spans[idx - 1]));

            break;
        }

        let (rhs, err) = parse_value(
            working_set,
            spans[idx],
            &SyntaxShape::Any,
            expand_aliases_denylist,
        );
        error = error.or(err);

        while op_prec <= last_prec && expr_stack.len() > 1 {
            // Collapse the right associated operations first
            // so that we can get back to a stack with a lower precedence
            let mut rhs = expr_stack
                .pop()
                .expect("internal error: expression stack empty");
            let mut op = expr_stack
                .pop()
                .expect("internal error: expression stack empty");

            last_prec = op.precedence();

            if last_prec < op_prec {
                expr_stack.push(op);
                expr_stack.push(rhs);
                break;
            }

            let mut lhs = expr_stack
                .pop()
                .expect("internal error: expression stack empty");

            if let Some(row_var_id) = lhs_row_var_id {
                expand_to_cell_path(working_set, &mut lhs, row_var_id, expand_aliases_denylist);
            }

            let (result_ty, err) = math_result_type(working_set, &mut lhs, &mut op, &mut rhs);
            error = error.or(err);

            let op_span = span(&[lhs.span, rhs.span]);
            expr_stack.push(Expression {
                expr: Expr::BinaryOp(Box::new(lhs), Box::new(op), Box::new(rhs)),
                span: op_span,
                ty: result_ty,
                custom_completion: None,
            });
        }
        expr_stack.push(op);
        expr_stack.push(rhs);

        last_prec = op_prec;

        idx += 1;
    }

    while expr_stack.len() != 1 {
        let mut rhs = expr_stack
            .pop()
            .expect("internal error: expression stack empty");
        let mut op = expr_stack
            .pop()
            .expect("internal error: expression stack empty");
        let mut lhs = expr_stack
            .pop()
            .expect("internal error: expression stack empty");

        if let Some(row_var_id) = lhs_row_var_id {
            expand_to_cell_path(working_set, &mut lhs, row_var_id, expand_aliases_denylist);
        }

        let (result_ty, err) = math_result_type(working_set, &mut lhs, &mut op, &mut rhs);
        error = error.or(err);

        let binary_op_span = span(&[lhs.span, rhs.span]);
        expr_stack.push(Expression {
            expr: Expr::BinaryOp(Box::new(lhs), Box::new(op), Box::new(rhs)),
            span: binary_op_span,
            ty: result_ty,
            custom_completion: None,
        });
    }

    let output = expr_stack
        .pop()
        .expect("internal error: expression stack empty");

    (output, error)
}

pub fn parse_expression(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    let mut pos = 0;
    let mut shorthand = vec![];

    while pos < spans.len() {
        // Check if there is any environment shorthand
        let name = working_set.get_span_contents(spans[pos]);

        let split = name.splitn(2, |x| *x == b'=');
        let split: Vec<_> = split.collect();
        if split.len() == 2 && !split[0].is_empty() {
            let point = split[0].len() + 1;

            let lhs = parse_string_strict(
                working_set,
                Span {
                    start: spans[pos].start,
                    end: spans[pos].start + point - 1,
                },
            );
            let rhs = if spans[pos].start + point < spans[pos].end {
                let rhs_span = Span {
                    start: spans[pos].start + point,
                    end: spans[pos].end,
                };

                if working_set.get_span_contents(rhs_span).starts_with(b"$") {
                    parse_dollar_expr(working_set, rhs_span, expand_aliases_denylist)
                } else {
                    parse_string_strict(working_set, rhs_span)
                }
            } else {
                (
                    Expression {
                        expr: Expr::String(String::new()),
                        span: Span { start: 0, end: 0 },
                        ty: Type::Nothing,
                        custom_completion: None,
                    },
                    None,
                )
            };

            if lhs.1.is_none() && rhs.1.is_none() {
                shorthand.push((lhs.0, rhs.0));
                pos += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if pos == spans.len() {
        return (
            garbage(span(spans)),
            Some(ParseError::UnknownCommand(spans[0])),
        );
    }

    let (output, err) = if is_math_expression_like(working_set, spans[pos], expand_aliases_denylist)
    {
        parse_math_expression(working_set, &spans[pos..], None, expand_aliases_denylist)
    } else {
        let bytes = working_set.get_span_contents(spans[pos]);

        // For now, check for special parses of certain keywords
        match bytes {
            b"def" => (
                parse_call(
                    working_set,
                    &spans[pos..],
                    spans[0],
                    expand_aliases_denylist,
                )
                .0,
                Some(ParseError::BuiltinCommandInPipeline("def".into(), spans[0])),
            ),
            b"extern" => (
                parse_call(
                    working_set,
                    &spans[pos..],
                    spans[0],
                    expand_aliases_denylist,
                )
                .0,
                Some(ParseError::BuiltinCommandInPipeline(
                    "extern".into(),
                    spans[0],
                )),
            ),
            b"for" => (
                parse_call(
                    working_set,
                    &spans[pos..],
                    spans[0],
                    expand_aliases_denylist,
                )
                .0,
                Some(ParseError::BuiltinCommandInPipeline("for".into(), spans[0])),
            ),
            b"let" => (
                parse_call(
                    working_set,
                    &spans[pos..],
                    spans[0],
                    expand_aliases_denylist,
                )
                .0,
                Some(ParseError::LetInPipeline(
                    String::from_utf8_lossy(match spans.len() {
                        1 | 2 | 3 => b"value",
                        _ => working_set.get_span_contents(spans[3]),
                    })
                    .to_string(),
                    String::from_utf8_lossy(match spans.len() {
                        1 => b"variable",
                        _ => working_set.get_span_contents(spans[1]),
                    })
                    .to_string(),
                    spans[0],
                )),
            ),
            b"mut" => (
                parse_call(
                    working_set,
                    &spans[pos..],
                    spans[0],
                    expand_aliases_denylist,
                )
                .0,
                Some(ParseError::MutInPipeline(
                    String::from_utf8_lossy(match spans.len() {
                        1 | 2 | 3 => b"value",
                        _ => working_set.get_span_contents(spans[3]),
                    })
                    .to_string(),
                    String::from_utf8_lossy(match spans.len() {
                        1 => b"variable",
                        _ => working_set.get_span_contents(spans[1]),
                    })
                    .to_string(),
                    spans[0],
                )),
            ),
            b"alias" => (
                parse_call(
                    working_set,
                    &spans[pos..],
                    spans[0],
                    expand_aliases_denylist,
                )
                .0,
                Some(ParseError::BuiltinCommandInPipeline(
                    "alias".into(),
                    spans[0],
                )),
            ),
            b"module" => (
                parse_call(
                    working_set,
                    &spans[pos..],
                    spans[0],
                    expand_aliases_denylist,
                )
                .0,
                Some(ParseError::BuiltinCommandInPipeline(
                    "module".into(),
                    spans[0],
                )),
            ),
            b"use" => (
                parse_call(
                    working_set,
                    &spans[pos..],
                    spans[0],
                    expand_aliases_denylist,
                )
                .0,
                Some(ParseError::BuiltinCommandInPipeline("use".into(), spans[0])),
            ),
            b"overlay" => {
                if spans.len() > 1 && working_set.get_span_contents(spans[1]) == b"list" {
                    // whitelist 'overlay list'
                    parse_call(
                        working_set,
                        &spans[pos..],
                        spans[0],
                        expand_aliases_denylist,
                    )
                } else {
                    (
                        parse_call(
                            working_set,
                            &spans[pos..],
                            spans[0],
                            expand_aliases_denylist,
                        )
                        .0,
                        Some(ParseError::BuiltinCommandInPipeline(
                            "overlay".into(),
                            spans[0],
                        )),
                    )
                }
            }
            b"source" => (
                parse_call(
                    working_set,
                    &spans[pos..],
                    spans[0],
                    expand_aliases_denylist,
                )
                .0,
                Some(ParseError::BuiltinCommandInPipeline(
                    "source".into(),
                    spans[0],
                )),
            ),
            b"export" => (
                parse_call(
                    working_set,
                    &spans[pos..],
                    spans[0],
                    expand_aliases_denylist,
                )
                .0,
                Some(ParseError::UnexpectedKeyword("export".into(), spans[0])),
            ),
            b"hide" => (
                parse_call(
                    working_set,
                    &spans[pos..],
                    spans[0],
                    expand_aliases_denylist,
                )
                .0,
                Some(ParseError::BuiltinCommandInPipeline(
                    "hide".into(),
                    spans[0],
                )),
            ),
            #[cfg(feature = "plugin")]
            b"register" => (
                parse_call(
                    working_set,
                    &spans[pos..],
                    spans[0],
                    expand_aliases_denylist,
                )
                .0,
                Some(ParseError::BuiltinCommandInPipeline(
                    "plugin".into(),
                    spans[0],
                )),
            ),

            _ => parse_call(
                working_set,
                &spans[pos..],
                spans[0],
                expand_aliases_denylist,
            ),
        }
    };

    let with_env = working_set.find_decl(b"with-env", &Type::Any);

    if !shorthand.is_empty() {
        if let Some(decl_id) = with_env {
            let mut block = Block::default();
            let ty = output.ty.clone();
            block.pipelines = vec![Pipeline {
                expressions: vec![output],
            }];

            let block_id = working_set.add_block(block);

            let mut env_vars = vec![];
            for sh in shorthand {
                env_vars.push(sh.0);
                env_vars.push(sh.1);
            }

            let arguments = vec![
                Argument::Positional(Expression {
                    expr: Expr::List(env_vars),
                    span: span(&spans[..pos]),
                    ty: Type::Any,
                    custom_completion: None,
                }),
                Argument::Positional(Expression {
                    expr: Expr::Closure(block_id),
                    span: span(&spans[pos..]),
                    ty: Type::Closure,
                    custom_completion: None,
                }),
            ];

            let expr = Expr::Call(Box::new(Call {
                head: Span { start: 0, end: 0 },
                decl_id,
                arguments,
                redirect_stdout: true,
                redirect_stderr: false,
            }));

            (
                Expression {
                    expr,
                    custom_completion: None,
                    span: span(spans),
                    ty,
                },
                err,
            )
        } else {
            (output, err)
        }
    } else {
        (output, err)
    }
}

pub fn parse_variable(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Option<VarId>, Option<ParseError>) {
    let bytes = working_set.get_span_contents(span);

    if is_variable(bytes) {
        if let Some(var_id) = working_set.find_variable(bytes) {
            let input = working_set.get_variable(var_id).ty.clone();
            working_set.type_scope.add_type(input);

            (Some(var_id), None)
        } else {
            (None, None)
        }
    } else {
        (
            None,
            Some(ParseError::Expected("valid variable name".into(), span)),
        )
    }
}

pub fn parse_builtin_commands(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
    expand_aliases_denylist: &[usize],
) -> (Pipeline, Option<ParseError>) {
    let name = working_set.get_span_contents(lite_command.parts[0]);

    match name {
        b"def" | b"def-env" => parse_def(working_set, lite_command, expand_aliases_denylist),
        b"extern" => parse_extern(working_set, lite_command, expand_aliases_denylist),
        b"let" => parse_let(working_set, &lite_command.parts, expand_aliases_denylist),
        b"mut" => parse_mut(working_set, &lite_command.parts, expand_aliases_denylist),
        b"for" => {
            let (expr, err) = parse_for(working_set, &lite_command.parts, expand_aliases_denylist);
            (Pipeline::from_vec(vec![expr]), err)
        }
        b"alias" => parse_alias(working_set, &lite_command.parts, expand_aliases_denylist),
        b"module" => parse_module(working_set, &lite_command.parts, expand_aliases_denylist),
        b"use" => {
            let (pipeline, _, err) =
                parse_use(working_set, &lite_command.parts, expand_aliases_denylist);
            (pipeline, err)
        }
        b"overlay" => parse_overlay(working_set, &lite_command.parts, expand_aliases_denylist),
        b"source" | b"source-env" => {
            parse_source(working_set, &lite_command.parts, expand_aliases_denylist)
        }
        b"export" => parse_export_in_block(working_set, lite_command, expand_aliases_denylist),
        b"hide" => parse_hide(working_set, &lite_command.parts, expand_aliases_denylist),
        #[cfg(feature = "plugin")]
        b"register" => parse_register(working_set, &lite_command.parts, expand_aliases_denylist),
        _ => {
            let (expr, err) =
                parse_expression(working_set, &lite_command.parts, expand_aliases_denylist);
            (Pipeline::from_vec(vec![expr]), err)
        }
    }
}

pub fn parse_record(
    working_set: &mut StateWorkingSet,
    span: Span,
    expand_aliases_denylist: &[usize],
) -> (Expression, Option<ParseError>) {
    let bytes = working_set.get_span_contents(span);

    let mut error = None;
    let mut start = span.start;
    let mut end = span.end;

    if bytes.starts_with(b"{") {
        start += 1;
    } else {
        error = error.or_else(|| {
            Some(ParseError::Expected(
                "{".into(),
                Span {
                    start,
                    end: start + 1,
                },
            ))
        });
    }

    if bytes.ends_with(b"}") {
        end -= 1;
    } else {
        error = error.or_else(|| Some(ParseError::Unclosed("}".into(), Span { start: end, end })));
    }

    let inner_span = Span { start, end };
    let source = working_set.get_span_contents(inner_span);

    let (tokens, err) = lex(source, start, &[b'\n', b'\r', b','], &[b':'], true);
    error = error.or(err);

    let mut output = vec![];
    let mut idx = 0;

    while idx < tokens.len() {
        let (field, err) = parse_value(
            working_set,
            tokens[idx].span,
            &SyntaxShape::Any,
            expand_aliases_denylist,
        );
        error = error.or(err);

        idx += 1;
        if idx == tokens.len() {
            return (
                garbage(span),
                Some(ParseError::Expected("record".into(), span)),
            );
        }
        let colon = working_set.get_span_contents(tokens[idx].span);
        idx += 1;
        if idx == tokens.len() || colon != b":" {
            //FIXME: need better error
            return (
                garbage(span),
                Some(ParseError::Expected("record".into(), span)),
            );
        }
        let (value, err) = parse_value(
            working_set,
            tokens[idx].span,
            &SyntaxShape::Any,
            expand_aliases_denylist,
        );
        error = error.or(err);
        idx += 1;

        output.push((field, value));
    }

    (
        Expression {
            expr: Expr::Record(output),
            span,
            ty: Type::Any, //FIXME: but we don't know the contents of the fields, do we?
            custom_completion: None,
        },
        error,
    )
}

pub fn parse_block(
    working_set: &mut StateWorkingSet,
    lite_block: &LiteBlock,
    scoped: bool,
    expand_aliases_denylist: &[usize],
    is_subexpression: bool,
) -> (Block, Option<ParseError>) {
    trace!("parsing block: {:?}", lite_block);

    if scoped {
        working_set.enter_scope();
    }
    working_set.type_scope.enter_scope();

    let mut error = None;

    // Pre-declare any definition so that definitions
    // that share the same block can see each other
    for pipeline in &lite_block.block {
        if pipeline.commands.len() == 1 {
            if let Some(err) = parse_def_predecl(
                working_set,
                &pipeline.commands[0].parts,
                expand_aliases_denylist,
            ) {
                error = error.or(Some(err));
            }
        }
    }

    let block: Block = lite_block
        .block
        .iter()
        .enumerate()
        .map(|(idx, pipeline)| {
            if pipeline.commands.len() > 1 {
                let mut output = pipeline
                    .commands
                    .iter()
                    .map(|command| {
                        let (expr, err) =
                            parse_expression(working_set, &command.parts, expand_aliases_denylist);

                        working_set.type_scope.add_type(expr.ty.clone());

                        if error.is_none() {
                            error = err;
                        }

                        expr
                    })
                    .collect::<Vec<Expression>>();

                if is_subexpression {
                    for expr in output.iter_mut().skip(1) {
                        if expr.has_in_variable(working_set) {
                            *expr = wrap_expr_with_collect(working_set, expr);
                        }
                    }
                } else {
                    for expr in output.iter_mut() {
                        if expr.has_in_variable(working_set) {
                            *expr = wrap_expr_with_collect(working_set, expr);
                        }
                    }
                }

                Pipeline {
                    expressions: output,
                }
            } else {
                let (mut pipeline, err) = parse_builtin_commands(
                    working_set,
                    &pipeline.commands[0],
                    expand_aliases_denylist,
                );

                if idx == 0 {
                    if let Some(let_decl_id) = working_set.find_decl(b"let", &Type::Any) {
                        if let Some(let_env_decl_id) = working_set.find_decl(b"let-env", &Type::Any)
                        {
                            for expr in pipeline.expressions.iter_mut() {
                                if let Expression {
                                    expr: Expr::Call(call),
                                    ..
                                } = expr
                                {
                                    if call.decl_id == let_decl_id
                                        || call.decl_id == let_env_decl_id
                                    {
                                        // Do an expansion
                                        if let Some(Expression {
                                            expr: Expr::Keyword(_, _, expr),
                                            ..
                                        }) = call.positional_iter_mut().nth(1)
                                        {
                                            if expr.has_in_variable(working_set) {
                                                *expr = Box::new(wrap_expr_with_collect(
                                                    working_set,
                                                    expr,
                                                ));
                                            }
                                        }
                                        continue;
                                    } else if expr.has_in_variable(working_set) && !is_subexpression
                                    {
                                        *expr = wrap_expr_with_collect(working_set, expr);
                                    }
                                } else if expr.has_in_variable(working_set) && !is_subexpression {
                                    *expr = wrap_expr_with_collect(working_set, expr);
                                }
                            }
                        }
                    }
                }

                if error.is_none() {
                    error = err;
                }

                pipeline
            }
        })
        .into();

    if scoped {
        working_set.exit_scope();
    }
    working_set.type_scope.exit_scope();

    (block, error)
}

pub fn discover_captures_in_closure(
    working_set: &StateWorkingSet,
    block: &Block,
    seen: &mut Vec<VarId>,
    seen_blocks: &mut HashMap<BlockId, Vec<(VarId, Span)>>,
) -> Result<Vec<(VarId, Span)>, ParseError> {
    let mut output = vec![];

    for flag in &block.signature.named {
        if let Some(var_id) = flag.var_id {
            seen.push(var_id);
        }
    }

    for positional in &block.signature.required_positional {
        if let Some(var_id) = positional.var_id {
            seen.push(var_id);
        }
    }
    for positional in &block.signature.optional_positional {
        if let Some(var_id) = positional.var_id {
            seen.push(var_id);
        }
    }
    for positional in &block.signature.rest_positional {
        if let Some(var_id) = positional.var_id {
            seen.push(var_id);
        }
    }

    for pipeline in &block.pipelines {
        let result = discover_captures_in_pipeline(working_set, pipeline, seen, seen_blocks)?;
        output.extend(&result);
    }

    Ok(output)
}

fn discover_captures_in_pipeline(
    working_set: &StateWorkingSet,
    pipeline: &Pipeline,
    seen: &mut Vec<VarId>,
    seen_blocks: &mut HashMap<BlockId, Vec<(VarId, Span)>>,
) -> Result<Vec<(VarId, Span)>, ParseError> {
    let mut output = vec![];
    for expr in &pipeline.expressions {
        let result = discover_captures_in_expr(working_set, expr, seen, seen_blocks)?;
        output.extend(&result);
    }

    Ok(output)
}

// Closes over captured variables
pub fn discover_captures_in_expr(
    working_set: &StateWorkingSet,
    expr: &Expression,
    seen: &mut Vec<VarId>,
    seen_blocks: &mut HashMap<BlockId, Vec<(VarId, Span)>>,
) -> Result<Vec<(VarId, Span)>, ParseError> {
    let mut output: Vec<(VarId, Span)> = vec![];
    match &expr.expr {
        Expr::BinaryOp(lhs, _, rhs) => {
            let lhs_result = discover_captures_in_expr(working_set, lhs, seen, seen_blocks)?;
            let rhs_result = discover_captures_in_expr(working_set, rhs, seen, seen_blocks)?;

            output.extend(&lhs_result);
            output.extend(&rhs_result);
        }
        Expr::UnaryNot(expr) => {
            let result = discover_captures_in_expr(working_set, expr, seen, seen_blocks)?;
            output.extend(&result);
        }
        Expr::Closure(block_id) => {
            let block = working_set.get_block(*block_id);
            let results = {
                let mut seen = vec![];
                let results =
                    discover_captures_in_closure(working_set, block, &mut seen, seen_blocks)?;

                for (var_id, span) in results.iter() {
                    if !seen.contains(var_id) {
                        if let Some(variable) = working_set.get_variable_if_possible(*var_id) {
                            if variable.mutable {
                                return Err(ParseError::CaptureOfMutableVar(*span));
                            }
                        }
                    }
                }

                results
            };
            seen_blocks.insert(*block_id, results.clone());
            for (var_id, span) in results.into_iter() {
                if !seen.contains(&var_id) {
                    output.push((var_id, span))
                }
            }
        }
        Expr::Block(block_id) => {
            let block = working_set.get_block(*block_id);
            // FIXME: is this correct?
            let results = {
                let mut seen = vec![];
                discover_captures_in_closure(working_set, block, &mut seen, seen_blocks)?
            };
            seen_blocks.insert(*block_id, results.clone());
            for (var_id, span) in results.into_iter() {
                if !seen.contains(&var_id) {
                    output.push((var_id, span))
                }
            }
        }
        Expr::Binary(_) => {}
        Expr::Bool(_) => {}
        Expr::Call(call) => {
            let decl = working_set.get_decl(call.decl_id);
            if let Some(block_id) = decl.get_block_id() {
                match seen_blocks.get(&block_id) {
                    Some(capture_list) => {
                        output.extend(capture_list);
                    }
                    None => {
                        let block = working_set.get_block(block_id);
                        if !block.captures.is_empty() {
                            output.extend(block.captures.iter().map(|var_id| (*var_id, call.head)));
                        } else {
                            let mut seen = vec![];
                            seen_blocks.insert(block_id, output.clone());

                            let result = discover_captures_in_closure(
                                working_set,
                                block,
                                &mut seen,
                                seen_blocks,
                            )?;
                            output.extend(&result);
                            seen_blocks.insert(block_id, result);
                        }
                    }
                }
            }

            for named in call.named_iter() {
                if let Some(arg) = &named.2 {
                    let result = discover_captures_in_expr(working_set, arg, seen, seen_blocks)?;
                    output.extend(&result);
                }
            }

            for positional in call.positional_iter() {
                let result = discover_captures_in_expr(working_set, positional, seen, seen_blocks)?;
                output.extend(&result);
            }
        }
        Expr::CellPath(_) => {}
        Expr::DateTime(_) => {}
        Expr::ExternalCall(head, exprs) => {
            let result = discover_captures_in_expr(working_set, head, seen, seen_blocks)?;
            output.extend(&result);

            for expr in exprs {
                let result = discover_captures_in_expr(working_set, expr, seen, seen_blocks)?;
                output.extend(&result);
            }
        }
        Expr::Filepath(_) => {}
        Expr::Directory(_) => {}
        Expr::Float(_) => {}
        Expr::FullCellPath(cell_path) => {
            let result =
                discover_captures_in_expr(working_set, &cell_path.head, seen, seen_blocks)?;
            output.extend(&result);
        }
        Expr::ImportPattern(_) => {}
        Expr::Overlay(_) => {}
        Expr::Garbage => {}
        Expr::Nothing => {}
        Expr::GlobPattern(_) => {}
        Expr::Int(_) => {}
        Expr::Keyword(_, _, expr) => {
            let result = discover_captures_in_expr(working_set, expr, seen, seen_blocks)?;
            output.extend(&result);
        }
        Expr::List(exprs) => {
            for expr in exprs {
                let result = discover_captures_in_expr(working_set, expr, seen, seen_blocks)?;
                output.extend(&result);
            }
        }
        Expr::Operator(_) => {}
        Expr::Range(expr1, expr2, expr3, _) => {
            if let Some(expr) = expr1 {
                let result = discover_captures_in_expr(working_set, expr, seen, seen_blocks)?;
                output.extend(&result);
            }
            if let Some(expr) = expr2 {
                let result = discover_captures_in_expr(working_set, expr, seen, seen_blocks)?;
                output.extend(&result);
            }
            if let Some(expr) = expr3 {
                let result = discover_captures_in_expr(working_set, expr, seen, seen_blocks)?;
                output.extend(&result);
            }
        }
        Expr::Record(fields) => {
            for (field_name, field_value) in fields {
                output.extend(&discover_captures_in_expr(
                    working_set,
                    field_name,
                    seen,
                    seen_blocks,
                )?);
                output.extend(&discover_captures_in_expr(
                    working_set,
                    field_value,
                    seen,
                    seen_blocks,
                )?);
            }
        }
        Expr::Signature(sig) => {
            // Something with a declaration, similar to a var decl, will introduce more VarIds into the stack at eval
            for pos in &sig.required_positional {
                if let Some(var_id) = pos.var_id {
                    seen.push(var_id);
                }
            }
            for pos in &sig.optional_positional {
                if let Some(var_id) = pos.var_id {
                    seen.push(var_id);
                }
            }
            if let Some(rest) = &sig.rest_positional {
                if let Some(var_id) = rest.var_id {
                    seen.push(var_id);
                }
            }
            for named in &sig.named {
                if let Some(var_id) = named.var_id {
                    seen.push(var_id);
                }
            }
        }
        Expr::String(_) => {}
        Expr::StringInterpolation(exprs) => {
            for expr in exprs {
                let result = discover_captures_in_expr(working_set, expr, seen, seen_blocks)?;
                output.extend(&result);
            }
        }
        Expr::RowCondition(block_id) | Expr::Subexpression(block_id) => {
            let block = working_set.get_block(*block_id);
            let results = {
                let mut seen = vec![];
                discover_captures_in_closure(working_set, block, &mut seen, seen_blocks)?
            };
            seen_blocks.insert(*block_id, results.clone());
            for (var_id, span) in results.into_iter() {
                if !seen.contains(&var_id) {
                    output.push((var_id, span))
                }
            }
        }
        Expr::Table(headers, values) => {
            for header in headers {
                let result = discover_captures_in_expr(working_set, header, seen, seen_blocks)?;
                output.extend(&result);
            }
            for row in values {
                for cell in row {
                    let result = discover_captures_in_expr(working_set, cell, seen, seen_blocks)?;
                    output.extend(&result);
                }
            }
        }
        Expr::ValueWithUnit(expr, _) => {
            let result = discover_captures_in_expr(working_set, expr, seen, seen_blocks)?;
            output.extend(&result);
        }
        Expr::Var(var_id) => {
            if (*var_id > ENV_VARIABLE_ID || *var_id == IN_VARIABLE_ID) && !seen.contains(var_id) {
                output.push((*var_id, expr.span));
            }
        }
        Expr::VarDecl(var_id) => {
            seen.push(*var_id);
        }
    }
    Ok(output)
}

fn wrap_expr_with_collect(working_set: &mut StateWorkingSet, expr: &Expression) -> Expression {
    let span = expr.span;

    if let Some(decl_id) = working_set.find_decl(b"collect", &Type::Any) {
        let mut output = vec![];

        let var_id = working_set.next_var_id();
        let mut signature = Signature::new("");
        signature.required_positional.push(PositionalArg {
            var_id: Some(var_id),
            name: "$in".into(),
            desc: String::new(),
            shape: SyntaxShape::Any,
            default_value: None,
        });

        let mut expr = expr.clone();
        expr.replace_in_variable(working_set, var_id);

        let block = Block {
            pipelines: vec![Pipeline {
                expressions: vec![expr],
            }],
            signature: Box::new(signature),
            ..Default::default()
        };

        let block_id = working_set.add_block(block);

        output.push(Argument::Positional(Expression {
            expr: Expr::Closure(block_id),
            span,
            ty: Type::Any,
            custom_completion: None,
        }));

        output.push(Argument::Named((
            Spanned {
                item: "keep-env".to_string(),
                span: Span::new(0, 0),
            },
            None,
            None,
        )));

        // The containing, synthetic call to `collect`.
        // We don't want to have a real span as it will confuse flattening
        // The args are where we'll get the real info
        Expression {
            expr: Expr::Call(Box::new(Call {
                head: Span::new(0, 0),
                arguments: output,
                decl_id,
                redirect_stdout: true,
                redirect_stderr: false,
            })),
            span,
            ty: Type::String,
            custom_completion: None,
        }
    } else {
        Expression::garbage(span)
    }
}

// Parses a vector of u8 to create an AST Block. If a file name is given, then
// the name is stored in the working set. When parsing a source without a file
// name, the source of bytes is stored as "source"
pub fn parse(
    working_set: &mut StateWorkingSet,
    fname: Option<&str>,
    contents: &[u8],
    scoped: bool,
    expand_aliases_denylist: &[usize],
) -> (Block, Option<ParseError>) {
    trace!("starting top-level parse");

    let mut error = None;

    let span_offset = working_set.next_span_start();

    let name = match fname {
        Some(fname) => fname.to_string(),
        None => "source".to_string(),
    };

    working_set.add_file(name, contents);

    let (output, err) = lex(contents, span_offset, &[], &[], false);
    error = error.or(err);

    let (output, err) = lite_parse(&output);
    error = error.or(err);

    let (mut output, err) =
        parse_block(working_set, &output, scoped, expand_aliases_denylist, false);
    error = error.or(err);

    let mut seen = vec![];
    let mut seen_blocks = HashMap::new();

    let captures = discover_captures_in_closure(working_set, &output, &mut seen, &mut seen_blocks);
    match captures {
        Ok(captures) => output.captures = captures.into_iter().map(|(var_id, _)| var_id).collect(),
        Err(err) => error = Some(err),
    }

    // Also check other blocks that might have been imported
    for (block_idx, block) in working_set.delta.blocks.iter().enumerate() {
        let block_id = block_idx + working_set.permanent_state.num_blocks();

        if !seen_blocks.contains_key(&block_id) {
            let captures =
                discover_captures_in_closure(working_set, block, &mut seen, &mut seen_blocks);
            match captures {
                Ok(captures) => {
                    seen_blocks.insert(block_id, captures);
                }
                Err(err) => error = Some(err),
            }
        }
    }

    for (block_id, captures) in seen_blocks.into_iter() {
        // In theory, we should only be updating captures where we have new information
        // the only place where this is possible would be blocks that are newly created
        // by our working set delta. If we ever tried to modify the permanent state, we'd
        // panic (again, in theory, this shouldn't be possible)
        let block = working_set.get_block(block_id);
        let block_captures_empty = block.captures.is_empty();
        if !captures.is_empty() && block_captures_empty {
            let block = working_set.get_block_mut(block_id);
            block.captures = captures.into_iter().map(|(var_id, _)| var_id).collect();
        }
    }

    (output, error)
}
