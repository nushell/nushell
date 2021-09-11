use crate::{
    lex, lite_parse,
    type_check::{math_result_type, type_compatible},
    LiteBlock, ParseError, Token, TokenContents,
};

use nu_protocol::{
    ast::{
        Block, Call, Expr, Expression, FullCellPath, Operator, PathMember, Pipeline,
        RangeInclusion, RangeOperator, Statement,
    },
    engine::StateWorkingSet,
    span, Flag, PositionalArg, Signature, Span, SyntaxShape, Type, VarId,
};

#[derive(Debug, Clone)]
pub enum Import {}

#[derive(Debug, Clone)]
pub struct VarDecl {
    var_id: VarId,
    expression: Expression,
}

fn garbage(span: Span) -> Expression {
    Expression::garbage(span)
}

fn garbage_statement(spans: &[Span]) -> Statement {
    Statement::Pipeline(Pipeline::from_vec(vec![garbage(span(spans))]))
}

fn is_identifier_byte(b: u8) -> bool {
    b != b'.' && b != b'[' && b != b'(' && b != b'{'
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

fn check_call(command: Span, sig: &Signature, call: &Call) -> Option<ParseError> {
    if call.positional.len() < sig.required_positional.len() {
        let missing = &sig.required_positional[call.positional.len()];
        Some(ParseError::MissingPositional(missing.name.clone(), command))
    } else {
        for req_flag in sig.named.iter().filter(|x| x.required) {
            if call.named.iter().all(|(n, _)| n != &req_flag.long) {
                return Some(ParseError::MissingRequiredFlag(
                    req_flag.long.clone(),
                    command,
                ));
            }
        }
        None
    }
}

fn check_name(working_set: &mut StateWorkingSet, spans: &[Span]) -> Option<ParseError> {
    if spans[1..].len() < 2 {
        Some(ParseError::UnknownState(
            "missing definition name".into(),
            span(spans),
        ))
    } else if working_set.get_span_contents(spans[2]) != b"=" {
        Some(ParseError::UnknownState(
            "missing equal sign in definition".into(),
            span(spans),
        ))
    } else {
        None
    }
}

pub fn parse_external_call(
    _working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> (Expression, Option<ParseError>) {
    // TODO: add external parsing
    let mut args = vec![];
    let name = spans[0];
    for span in &spans[1..] {
        args.push(*span);
    }
    (
        Expression {
            expr: Expr::ExternalCall(name, args),
            span: span(spans),
            ty: Type::Unknown,
        },
        None,
    )
}

fn parse_long_flag(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    spans_idx: &mut usize,
    sig: &Signature,
) -> (Option<String>, Option<Expression>, Option<ParseError>) {
    let arg_span = spans[*spans_idx];
    let arg_contents = working_set.get_span_contents(arg_span);

    if arg_contents.starts_with(b"--") {
        // FIXME: only use the first you find
        let split: Vec<_> = arg_contents.split(|x| *x == b'=').collect();
        let long_name = String::from_utf8(split[0].into());
        if let Ok(long_name) = long_name {
            if let Some(flag) = sig.get_long_flag(&long_name) {
                if let Some(arg_shape) = &flag.arg {
                    if split.len() > 1 {
                        // and we also have the argument
                        let mut span = arg_span;
                        span.start += long_name.len() + 1; //offset by long flag and '='
                        let (arg, err) = parse_value(working_set, span, arg_shape);

                        (Some(long_name), Some(arg), err)
                    } else if let Some(arg) = spans.get(*spans_idx + 1) {
                        let (arg, err) = parse_value(working_set, *arg, arg_shape);

                        *spans_idx += 1;
                        (Some(long_name), Some(arg), err)
                    } else {
                        (
                            Some(long_name),
                            None,
                            Some(ParseError::MissingFlagParam(arg_span)),
                        )
                    }
                } else {
                    // A flag with no argument
                    (Some(long_name), None, None)
                }
            } else {
                (
                    Some(long_name),
                    None,
                    Some(ParseError::UnknownFlag(arg_span)),
                )
            }
        } else {
            (Some("--".into()), None, Some(ParseError::NonUtf8(arg_span)))
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
                        error = error.or(Some(ParseError::UnknownFlag(*first)));
                    }
                } else if let Some(first) = unmatched_short_flags.first() {
                    error = error.or(Some(ParseError::UnknownFlag(*first)));
                }
            } else if let Some(first) = unmatched_short_flags.first() {
                error = error.or(Some(ParseError::UnknownFlag(*first)));
            }
        } else if !unmatched_short_flags.is_empty() {
            if let Some(first) = unmatched_short_flags.first() {
                error = error.or(Some(ParseError::UnknownFlag(*first)));
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
            if positional_idx < signature.required_positional.len()
                && spans.len() > (signature.required_positional.len() - positional_idx)
            {
                spans.len() - (signature.required_positional.len() - positional_idx - 1)
            } else if signature.num_positionals_after(positional_idx) == 0 {
                spans.len()
            } else {
                spans_idx + 1
            }
        }
    }
}

fn parse_multispan_value(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    spans_idx: &mut usize,
    shape: &SyntaxShape,
) -> (Expression, Option<ParseError>) {
    let mut error = None;

    match shape {
        SyntaxShape::VarWithOptType => {
            let (arg, err) = parse_var_with_opt_type(working_set, spans, spans_idx);
            error = error.or(err);

            (arg, error)
        }
        SyntaxShape::RowCondition => {
            let (arg, err) = parse_row_condition(working_set, &spans[*spans_idx..]);
            error = error.or(err);
            *spans_idx = spans.len() - 1;

            (arg, error)
        }
        SyntaxShape::Expression => {
            let (arg, err) = parse_expression(working_set, &spans[*spans_idx..]);
            error = error.or(err);
            *spans_idx = spans.len() - 1;

            (arg, error)
        }
        SyntaxShape::Keyword(keyword, arg) => {
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
                        String::from_utf8_lossy(keyword).into(),
                        spans[*spans_idx - 1],
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
                        ty: Type::Unknown,
                    },
                    error,
                );
            }
            let keyword_span = spans[*spans_idx - 1];
            let (expr, err) = parse_multispan_value(working_set, spans, spans_idx, arg);
            error = error.or(err);
            let ty = expr.ty.clone();

            (
                Expression {
                    expr: Expr::Keyword(keyword.clone(), keyword_span, Box::new(expr)),
                    span: arg_span,
                    ty,
                },
                error,
            )
        }
        _ => {
            // All other cases are single-span values
            let arg_span = spans[*spans_idx];

            let (arg, err) = parse_value(working_set, arg_span, shape);
            error = error.or(err);

            (arg, error)
        }
    }
}

pub fn parse_internal_call(
    working_set: &mut StateWorkingSet,
    command_span: Span,
    spans: &[Span],
    decl_id: usize,
) -> (Box<Call>, Span, Option<ParseError>) {
    let mut error = None;

    let mut call = Call::new();
    call.decl_id = decl_id;
    call.head = command_span;

    let signature = working_set.get_decl(decl_id).signature();

    // The index into the positional parameter in the definition
    let mut positional_idx = 0;

    // The index into the spans of argument data given to parse
    // Starting at the first argument
    let mut spans_idx = 0;

    while spans_idx < spans.len() {
        let arg_span = spans[spans_idx];

        // Check if we're on a long flag, if so, parse
        let (long_name, arg, err) = parse_long_flag(working_set, spans, &mut spans_idx, &signature);
        if let Some(long_name) = long_name {
            // We found a long flag, like --bar
            error = error.or(err);
            call.named.push((long_name, arg));
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

        if let Some(short_flags) = short_flags {
            error = error.or(err);
            for flag in short_flags {
                if let Some(arg_shape) = flag.arg {
                    if let Some(arg) = spans.get(spans_idx + 1) {
                        let (arg, err) = parse_value(working_set, *arg, &arg_shape);
                        error = error.or(err);

                        call.named.push((flag.long.clone(), Some(arg)));
                        spans_idx += 1;
                    } else {
                        error = error.or(Some(ParseError::MissingFlagParam(arg_span)))
                    }
                } else {
                    call.named.push((flag.long.clone(), None));
                }
            }
            spans_idx += 1;
            continue;
        }

        // Parse a positional arg if there is one
        if let Some(positional) = signature.get_positional(positional_idx) {
            let end = calculate_end_span(working_set, &signature, spans, spans_idx, positional_idx);

            // println!(
            //     "start: {} end: {} positional_idx: {}",
            //     spans_idx, end, positional_idx
            // );

            let orig_idx = spans_idx;
            let (arg, err) = parse_multispan_value(
                working_set,
                &spans[..end],
                &mut spans_idx,
                &positional.shape,
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
            call.positional.push(arg);
            positional_idx += 1;
        } else {
            call.positional.push(Expression::garbage(arg_span));
            error = error.or(Some(ParseError::ExtraPositional(arg_span)))
        }

        error = error.or(err);
        spans_idx += 1;
    }

    let err = check_call(command_span, &signature, &call);
    error = error.or(err);

    // FIXME: type unknown
    (Box::new(call), span(spans), error)
}

pub fn parse_call(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    expand_aliases: bool,
) -> (Expression, Option<ParseError>) {
    // assume spans.len() > 0?
    let mut pos = 0;
    let mut shorthand = vec![];

    while pos < spans.len() {
        // Check if there is any environment shorthand
        let name = working_set.get_span_contents(spans[pos]);
        let split: Vec<_> = name.splitn(2, |x| *x == b'=').collect();
        if split.len() == 2 {
            shorthand.push(split);
            pos += 1;
        } else {
            break;
        }
    }

    if pos == spans.len() {
        return (
            Expression::garbage(span(spans)),
            Some(ParseError::UnknownCommand(spans[0])),
        );
    }

    let name = working_set.get_span_contents(spans[pos]);

    let cmd_start = pos;

    if expand_aliases {
        if let Some(expansion) = working_set.find_alias(name) {
            let orig_span = spans[pos];
            //let mut spans = spans.to_vec();
            let mut new_spans: Vec<Span> = vec![];
            new_spans.extend(&spans[0..pos]);
            new_spans.extend(expansion);
            if spans.len() > pos {
                new_spans.extend(&spans[(pos + 1)..]);
            }

            let (result, err) = parse_call(working_set, &new_spans, false);

            let expression = match result {
                Expression {
                    expr: Expr::Call(mut call),
                    span,
                    ty,
                } => {
                    call.head = orig_span;
                    Expression {
                        expr: Expr::Call(call),
                        span,
                        ty,
                    }
                }
                x => x,
            };

            return (expression, err);
        }
    }

    pos += 1;

    if let Some(mut decl_id) = working_set.find_decl(name) {
        let mut name = name.to_vec();
        while pos < spans.len() {
            // look to see if it's a subcommand
            let mut new_name = name.to_vec();
            new_name.push(b' ');
            new_name.extend(working_set.get_span_contents(spans[pos]));

            if expand_aliases {
                if let Some(expansion) = working_set.find_alias(&new_name) {
                    let orig_span = span(&spans[cmd_start..pos + 1]);
                    //let mut spans = spans.to_vec();
                    let mut new_spans: Vec<Span> = vec![];
                    new_spans.extend(&spans[0..cmd_start]);
                    new_spans.extend(expansion);
                    if spans.len() > pos {
                        new_spans.extend(&spans[(pos + 1)..]);
                    }

                    let (result, err) = parse_call(working_set, &new_spans, false);

                    let expression = match result {
                        Expression {
                            expr: Expr::Call(mut call),
                            span,
                            ty,
                        } => {
                            call.head = orig_span;
                            Expression {
                                expr: Expr::Call(call),
                                span,
                                ty,
                            }
                        }
                        x => x,
                    };

                    return (expression, err);
                }
            }

            if let Some(did) = working_set.find_decl(&new_name) {
                decl_id = did;
            } else {
                break;
            }
            name = new_name;
            pos += 1;
        }

        // Before the internal parsing we check if there is no let or alias declarations
        // that are missing their name, e.g.: let = 1 or alias = 2
        if spans.len() > 1 {
            let test_equal = working_set.get_span_contents(spans[1]);

            if test_equal == &[b'='] {
                return (
                    garbage(Span::new(0, 0)),
                    Some(ParseError::UnknownState(
                        "internal error: incomplete statement".into(),
                        span(spans),
                    )),
                );
            }
        }

        // parse internal command
        let (call, _, err) =
            parse_internal_call(working_set, span(&spans[0..pos]), &spans[pos..], decl_id);
        (
            Expression {
                expr: Expr::Call(call),
                span: span(spans),
                ty: Type::Unknown, // FIXME
            },
            err,
        )
    } else {
        parse_external_call(working_set, spans)
    }
}

pub fn parse_int(token: &[u8], span: Span) -> (Expression, Option<ParseError>) {
    if let Some(token) = token.strip_prefix(b"0x") {
        if let Ok(v) = i64::from_str_radix(&String::from_utf8_lossy(token), 16) {
            (
                Expression {
                    expr: Expr::Int(v),
                    span,
                    ty: Type::Int,
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
) -> (Expression, Option<ParseError>) {
    // Range follows the following syntax: [<from>][<step_operator><step>]<range_operator>[<to>]
    //   where <step_operator> is ".."
    //   and  <range_operator> is ".." or "..<"
    //   and one of the <from> or <to> bounds must be present (just '..' is not allowed since it
    //     looks like parent directory)

    let contents = working_set.get_span_contents(span);
    let token = if let Ok(s) = String::from_utf8(contents.into()) {
        s
    } else {
        return (garbage(span), Some(ParseError::NonUtf8(span)));
    };

    // First, figure out what exact operators are used and determine their positions
    let dotdot_pos: Vec<_> = token.match_indices("..").map(|(pos, _)| pos).collect();

    let (step_op_pos, range_op_pos) =
        match dotdot_pos.len() {
            1 => (None, dotdot_pos[0]),
            2 => (Some(dotdot_pos[0]), dotdot_pos[1]),
            _ => return (
                garbage(span),
                Some(ParseError::Expected(
                    "one range operator ('..' or '..<') and optionally one step operator ('..')"
                        .into(),
                    span,
                )),
            ),
        };

    let _step_op_span = step_op_pos.map(|pos| {
        Span::new(
            span.start + pos,
            span.start + pos + "..".len(), // Only ".." is allowed for step operator
        )
    });

    let (range_op, range_op_str, range_op_span) = if let Some(pos) = token.find("..<") {
        if pos == range_op_pos {
            let op_str = "..<";
            let op_span = Span::new(
                span.start + range_op_pos,
                span.start + range_op_pos + op_str.len(),
            );
            (
                RangeOperator {
                    inclusion: RangeInclusion::RightExclusive,
                    span: op_span,
                },
                "..<",
                op_span,
            )
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
        (
            RangeOperator {
                inclusion: RangeInclusion::Inclusive,
                span: op_span,
            },
            "..",
            op_span,
        )
    };

    // Now, based on the operator positions, figure out where the bounds & step are located and
    // parse them
    // TODO: Actually parse the step number
    let from = if token.starts_with("..") {
        // token starts with either step operator, or range operator -- we don't care which one
        None
    } else {
        let from_span = Span::new(span.start, span.start + dotdot_pos[0]);
        match parse_value(working_set, from_span, &SyntaxShape::Number) {
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
        match parse_value(working_set, to_span, &SyntaxShape::Number) {
            (expression, None) => Some(Box::new(expression)),
            _ => {
                return (
                    garbage(span),
                    Some(ParseError::Expected("number".into(), span)),
                )
            }
        }
    };

    if let (None, None) = (&from, &to) {
        return (
            garbage(span),
            Some(ParseError::Expected(
                "at least one range bound set".into(),
                span,
            )),
        );
    }

    (
        Expression {
            expr: Expr::Range(from, to, range_op),
            span,
            ty: Type::Range,
        },
        None,
    )
}

pub(crate) fn parse_dollar_expr(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Expression, Option<ParseError>) {
    let contents = working_set.get_span_contents(span);

    if contents.starts_with(b"$\"") {
        parse_string_interpolation(working_set, span)
    } else if let (expr, None) = parse_range(working_set, span) {
        (expr, None)
    } else {
        parse_full_column_path(working_set, None, span)
    }
}

pub fn parse_string_interpolation(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Expression, Option<ParseError>) {
    #[derive(PartialEq, Eq, Debug)]
    enum InterpolationMode {
        String,
        Expression,
    }
    let mut error = None;

    let contents = working_set.get_span_contents(span);

    let start = if contents.starts_with(b"$\"") {
        span.start + 2
    } else {
        span.start
    };

    let end = if contents.ends_with(b"\"") && contents.len() > 2 {
        span.end - 1
    } else {
        span.end
    };

    let inner_span = Span { start, end };
    let contents = working_set.get_span_contents(inner_span).to_vec();

    let mut output = vec![];
    let mut mode = InterpolationMode::String;
    let mut token_start = start;
    let mut depth = 0;

    let mut b = start;

    #[allow(clippy::needless_range_loop)]
    while b != end {
        if contents[b - start] == b'(' && mode == InterpolationMode::String {
            depth = 1;
            mode = InterpolationMode::Expression;
            if token_start < b {
                let span = Span {
                    start: token_start,
                    end: b,
                };
                let str_contents = working_set.get_span_contents(span);
                output.push(Expression {
                    expr: Expr::String(String::from_utf8_lossy(str_contents).to_string()),
                    span,
                    ty: Type::String,
                });
            }
            token_start = b;
        } else if contents[b - start] == b'(' && mode == InterpolationMode::Expression {
            depth += 1;
        } else if contents[b - start] == b')' && mode == InterpolationMode::Expression {
            match depth {
                0 => {}
                1 => {
                    mode = InterpolationMode::String;

                    if token_start < b {
                        let span = Span {
                            start: token_start,
                            end: b + 1,
                        };

                        let (expr, err) = parse_full_column_path(working_set, None, span);
                        error = error.or(err);
                        output.push(expr);
                    }

                    token_start = b + 1;
                }
                _ => depth -= 1,
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
                output.push(Expression {
                    expr: Expr::String(String::from_utf8_lossy(str_contents).to_string()),
                    span,
                    ty: Type::String,
                });
            }
        }
        InterpolationMode::Expression => {
            if token_start < end {
                let span = Span {
                    start: token_start,
                    end,
                };

                let (expr, err) = parse_full_column_path(working_set, None, span);
                error = error.or(err);
                output.push(expr);
            }
        }
    }

    if let Some(decl_id) = working_set.find_decl(b"build-string") {
        (
            Expression {
                expr: Expr::Call(Box::new(Call {
                    head: Span {
                        start: span.start,
                        end: span.start + 2,
                    },
                    named: vec![],
                    positional: output,
                    decl_id,
                })),
                span,
                ty: Type::String,
            },
            error,
        )
    } else {
        (
            Expression::garbage(span),
            Some(ParseError::UnknownCommand(span)),
        )
    }
}

pub fn parse_variable_expr(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Expression, Option<ParseError>) {
    let contents = working_set.get_span_contents(span);

    if contents == b"$true" {
        return (
            Expression {
                expr: Expr::Bool(true),
                span,
                ty: Type::Bool,
            },
            None,
        );
    } else if contents == b"$false" {
        return (
            Expression {
                expr: Expr::Bool(false),
                span,
                ty: Type::Bool,
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
                    ty: working_set.get_variable(id).clone(),
                },
                None,
            )
        } else {
            let name = working_set.get_span_contents(span).to_vec();
            // this seems okay to set it to unknown here, but we should double-check
            let id = working_set.add_variable(name, Type::Unknown);
            (
                Expression {
                    expr: Expr::Var(id),
                    span,
                    ty: Type::Unknown,
                },
                None,
            )
        }
    } else {
        (garbage(span), err)
    }
}

pub fn parse_full_column_path(
    working_set: &mut StateWorkingSet,
    implicit_head: Option<VarId>,
    span: Span,
) -> (Expression, Option<ParseError>) {
    // FIXME: assume for now a paren expr, but needs more
    let full_column_span = span;
    let source = working_set.get_span_contents(span);
    let mut error = None;

    let (tokens, err) = lex(source, span.start, &[b'\n'], &[b'.']);
    error = error.or(err);

    let mut tokens = tokens.into_iter().peekable();
    if let Some(head) = tokens.peek() {
        let bytes = working_set.get_span_contents(head.span);
        let (head, mut expect_dot) = if bytes.starts_with(b"(") {
            let mut start = head.span.start;
            let mut end = head.span.end;

            if bytes.starts_with(b"(") {
                start += 1;
            }
            if bytes.ends_with(b")") {
                end -= 1;
            } else {
                error = error.or_else(|| {
                    Some(ParseError::Unclosed(
                        ")".into(),
                        Span {
                            start: end,
                            end: end + 1,
                        },
                    ))
                });
            }

            let span = Span { start, end };

            let source = working_set.get_span_contents(span);

            let (output, err) = lex(source, span.start, &[b'\n'], &[]);
            error = error.or(err);

            let (output, err) = lite_parse(&output);
            error = error.or(err);

            let (output, err) = parse_block(working_set, &output, true);
            error = error.or(err);

            let block_id = working_set.add_block(output);
            tokens.next();

            (
                Expression {
                    expr: Expr::Subexpression(block_id),
                    span,
                    ty: Type::Unknown, // FIXME
                },
                true,
            )
        } else if bytes.starts_with(b"$") {
            let (out, err) = parse_variable_expr(working_set, head.span);
            error = error.or(err);

            tokens.next();

            (out, true)
        } else if let Some(var_id) = implicit_head {
            (
                Expression {
                    expr: Expr::Var(var_id),
                    span: Span::unknown(),
                    ty: Type::Unknown,
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

        let mut tail = vec![];

        for path_element in tokens {
            let bytes = working_set.get_span_contents(path_element.span);

            if expect_dot {
                expect_dot = false;
                if bytes.len() != 1 || bytes[0] != b'.' {
                    error =
                        error.or_else(|| Some(ParseError::Expected('.'.into(), path_element.span)));
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
                        let (result, err) = parse_string(working_set, path_element.span);
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
                                error = error
                                    .or_else(|| Some(ParseError::Expected("string".into(), span)));
                            }
                        }
                    }
                }
            }
        }

        (
            Expression {
                expr: Expr::FullCellPath(Box::new(FullCellPath { head, tail })),
                ty: Type::Unknown,
                span: full_column_span,
            },
            error,
        )
    } else {
        (garbage(span), error)
    }
}

pub fn parse_string(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Expression, Option<ParseError>) {
    let bytes = working_set.get_span_contents(span);
    let bytes = if (bytes.starts_with(b"\"") && bytes.ends_with(b"\"") && bytes.len() > 1)
        || (bytes.starts_with(b"\'") && bytes.ends_with(b"\'") && bytes.len() > 1)
    {
        &bytes[1..(bytes.len() - 1)]
    } else {
        bytes
    };

    if let Ok(token) = String::from_utf8(bytes.into()) {
        (
            Expression {
                expr: Expr::String(token),
                span,
                ty: Type::String,
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

//TODO: Handle error case
pub fn parse_shape_name(
    _working_set: &StateWorkingSet,
    bytes: &[u8],
    span: Span,
) -> (SyntaxShape, Option<ParseError>) {
    let result = match bytes {
        b"any" => SyntaxShape::Any,
        b"string" => SyntaxShape::String,
        b"cell-path" => SyntaxShape::CellPath,
        b"number" => SyntaxShape::Number,
        b"range" => SyntaxShape::Range,
        b"int" => SyntaxShape::Int,
        b"path" => SyntaxShape::FilePath,
        b"glob" => SyntaxShape::GlobPattern,
        b"block" => SyntaxShape::Block,
        b"cond" => SyntaxShape::RowCondition,
        b"operator" => SyntaxShape::Operator,
        b"math" => SyntaxShape::MathExpression,
        b"variable" => SyntaxShape::Variable,
        b"signature" => SyntaxShape::Signature,
        b"expr" => SyntaxShape::Expression,
        _ => return (SyntaxShape::Any, Some(ParseError::UnknownType(span))),
    };

    (result, None)
}

pub fn parse_type(_working_set: &StateWorkingSet, bytes: &[u8]) -> Type {
    if bytes == b"int" {
        Type::Int
    } else {
        Type::Unknown
    }
}

pub fn parse_var_with_opt_type(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    spans_idx: &mut usize,
) -> (Expression, Option<ParseError>) {
    let bytes = working_set.get_span_contents(spans[*spans_idx]).to_vec();

    if bytes.ends_with(b":") {
        // We end with colon, so the next span should be the type
        if *spans_idx + 1 < spans.len() {
            *spans_idx += 1;
            let type_bytes = working_set.get_span_contents(spans[*spans_idx]);

            let ty = parse_type(working_set, type_bytes);

            let id = working_set.add_variable(bytes[0..(bytes.len() - 1)].to_vec(), ty.clone());

            (
                Expression {
                    expr: Expr::Var(id),
                    span: span(&spans[*spans_idx - 1..*spans_idx + 1]),
                    ty,
                },
                None,
            )
        } else {
            let id = working_set.add_variable(bytes[0..(bytes.len() - 1)].to_vec(), Type::Unknown);
            (
                Expression {
                    expr: Expr::Var(id),
                    span: spans[*spans_idx],
                    ty: Type::Unknown,
                },
                Some(ParseError::MissingType(spans[*spans_idx])),
            )
        }
    } else {
        let id = working_set.add_variable(bytes, Type::Unknown);

        (
            Expression {
                expr: Expr::Var(id),
                span: span(&spans[*spans_idx..*spans_idx + 1]),
                ty: Type::Unknown,
            },
            None,
        )
    }
}

pub fn expand_to_cell_path(
    working_set: &mut StateWorkingSet,
    expression: &mut Expression,
    var_id: VarId,
) {
    if let Expression {
        expr: Expr::String(_),
        span,
        ..
    } = expression
    {
        // Re-parse the string as if it were a cell-path
        let (new_expression, _err) = parse_full_column_path(working_set, Some(var_id), *span);

        *expression = new_expression;
    }
}

pub fn parse_row_condition(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> (Expression, Option<ParseError>) {
    let var_id = working_set.add_variable(b"$it".to_vec(), Type::Unknown);
    let (expression, err) = parse_math_expression(working_set, spans, Some(var_id));
    let span = span(spans);
    (
        Expression {
            ty: Type::Bool,
            span,
            expr: Expr::RowCondition(var_id, Box::new(expression)),
        },
        err,
    )
}

pub fn parse_signature(
    working_set: &mut StateWorkingSet,
    span: Span,
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
        error = error.or_else(|| {
            Some(ParseError::Unclosed(
                "]".into(),
                Span {
                    start: end,
                    end: end + 1,
                },
            ))
        });
    }

    let (sig, err) = parse_signature_helper(working_set, Span { start, end });
    error = error.or(err);

    (
        Expression {
            expr: Expr::Signature(sig),
            span,
            ty: Type::Unknown,
        },
        error,
    )
}

pub fn parse_signature_helper(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Box<Signature>, Option<ParseError>) {
    enum ParseMode {
        ArgMode,
        TypeMode,
    }

    enum Arg {
        Positional(PositionalArg, bool), // bool - required
        Flag(Flag),
    }

    let mut error = None;
    let source = working_set.get_span_contents(span);

    let (output, err) = lex(source, span.start, &[b'\n', b','], &[b':']);
    error = error.or(err);

    let mut args: Vec<Arg> = vec![];
    let mut rest_arg = None;
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
                        ParseMode::TypeMode => {
                            // We're seeing two types for the same thing for some reason, error
                            error =
                                error.or_else(|| Some(ParseError::Expected("type".into(), span)));
                        }
                    }
                } else {
                    match parse_mode {
                        ParseMode::ArgMode => {
                            if contents.starts_with(b"--") && contents.len() > 2 {
                                // Long flag
                                let flags: Vec<_> =
                                    contents.split(|x| x == &b'(').map(|x| x.to_vec()).collect();

                                let long = String::from_utf8_lossy(&flags[0]).to_string();
                                let variable_name = flags[0][2..].to_vec();
                                let var_id = working_set.add_variable(variable_name, Type::Unknown);

                                if flags.len() == 1 {
                                    args.push(Arg::Flag(Flag {
                                        arg: None,
                                        desc: String::new(),
                                        long,
                                        short: None,
                                        required: false,
                                        var_id: Some(var_id),
                                    }));
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
                                    let long = String::from_utf8_lossy(&flags[0]).to_string();
                                    let variable_name = flags[0][2..].to_vec();
                                    let var_id =
                                        working_set.add_variable(variable_name, Type::Unknown);

                                    if chars.len() == 1 {
                                        args.push(Arg::Flag(Flag {
                                            arg: None,
                                            desc: String::new(),
                                            long,
                                            short: Some(chars[0]),
                                            required: false,
                                            var_id: Some(var_id),
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

                                    args.push(Arg::Flag(Flag {
                                        arg: None,
                                        desc: String::new(),
                                        long: String::new(),
                                        short: None,
                                        required: false,
                                        var_id: None,
                                    }));
                                } else {
                                    let mut encoded_var_name = vec![0u8; 4];
                                    let len = chars[0].encode_utf8(&mut encoded_var_name).len();
                                    let variable_name = encoded_var_name[0..len].to_vec();
                                    let var_id =
                                        working_set.add_variable(variable_name, Type::Unknown);

                                    args.push(Arg::Flag(Flag {
                                        arg: None,
                                        desc: String::new(),
                                        long: String::new(),
                                        short: Some(chars[0]),
                                        required: false,
                                        var_id: Some(var_id),
                                    }));
                                }
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

                                let var_id = working_set.add_variable(contents, Type::Unknown);

                                // Positional arg, optional
                                args.push(Arg::Positional(
                                    PositionalArg {
                                        desc: String::new(),
                                        name,
                                        shape: SyntaxShape::Any,
                                        var_id: Some(var_id),
                                    },
                                    false,
                                ))
                            } else if let Some(contents) = contents.strip_prefix(b"...") {
                                let name = String::from_utf8_lossy(contents).to_string();
                                let contents_vec: Vec<u8> = contents.to_vec();

                                let var_id = working_set.add_variable(contents_vec, Type::Unknown);

                                if rest_arg.is_none() {
                                    rest_arg = Some(Arg::Positional(
                                        PositionalArg {
                                            desc: String::new(),
                                            name,
                                            shape: SyntaxShape::Any,
                                            var_id: Some(var_id),
                                        },
                                        false,
                                    ));
                                } else {
                                    error = error.or(Some(ParseError::MultipleRestParams(span)))
                                }
                            } else {
                                let name = String::from_utf8_lossy(contents).to_string();
                                let contents_vec = contents.to_vec();

                                let var_id = working_set.add_variable(contents_vec, Type::Unknown);

                                // Positional arg, required
                                args.push(Arg::Positional(
                                    PositionalArg {
                                        desc: String::new(),
                                        name,
                                        shape: SyntaxShape::Any,
                                        var_id: Some(var_id),
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
                                //TODO check if we're replacing one already
                                match last {
                                    Arg::Positional(PositionalArg { shape, var_id, .. }, ..) => {
                                        working_set.set_variable_type(var_id.expect("internal error: all custom parameters must have var_ids"), syntax_shape.to_type());
                                        *shape = syntax_shape;
                                    }
                                    Arg::Flag(Flag { arg, var_id, .. }) => {
                                        working_set.set_variable_type(var_id.expect("internal error: all custom parameters must have var_ids"), syntax_shape.to_type());
                                        *arg = Some(syntax_shape)
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
                    }
                }
            }
            _ => {}
        }
    }

    let mut sig = Signature::new(String::new());

    if let Some(Arg::Positional(positional, ..)) = rest_arg {
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
    for arg in args {
        match arg {
            Arg::Positional(positional, required) => {
                if required {
                    sig.required_positional.push(positional)
                } else {
                    sig.optional_positional.push(positional)
                }
            }
            Arg::Flag(flag) => sig.named.push(flag),
        }
    }

    (Box::new(sig), error)
}

pub fn parse_list_expression(
    working_set: &mut StateWorkingSet,
    span: Span,
    element_shape: &SyntaxShape,
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
        error = error.or_else(|| {
            Some(ParseError::Unclosed(
                "]".into(),
                Span {
                    start: end,
                    end: end + 1,
                },
            ))
        });
    }

    let span = Span { start, end };
    let source = working_set.get_span_contents(span);

    let (output, err) = lex(source, span.start, &[b'\n', b','], &[]);
    error = error.or(err);

    let (output, err) = lite_parse(&output);
    error = error.or(err);

    let mut args = vec![];

    let mut contained_type: Option<Type> = None;

    if !output.block.is_empty() {
        for arg in &output.block[0].commands {
            let mut spans_idx = 0;

            while spans_idx < arg.parts.len() {
                let (arg, err) =
                    parse_multispan_value(working_set, &arg.parts, &mut spans_idx, element_shape);
                error = error.or(err);

                if let Some(ref ctype) = contained_type {
                    if *ctype != arg.ty {
                        contained_type = Some(Type::Unknown);
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
                Type::Unknown
            })),
        },
        error,
    )
}

pub fn parse_table_expression(
    working_set: &mut StateWorkingSet,
    original_span: Span,
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
        error = error.or_else(|| {
            Some(ParseError::Unclosed(
                "]".into(),
                Span {
                    start: end,
                    end: end + 1,
                },
            ))
        });
    }

    let span = Span { start, end };

    let source = working_set.get_span_contents(span);

    let (output, err) = lex(source, start, &[b'\n', b','], &[]);
    error = error.or(err);

    let (output, err) = lite_parse(&output);
    error = error.or(err);

    match output.block.len() {
        0 => (
            Expression {
                expr: Expr::List(vec![]),
                span,
                ty: Type::List(Box::new(Type::Unknown)),
            },
            None,
        ),
        1 => {
            // List
            parse_list_expression(working_set, original_span, &SyntaxShape::Any)
        }
        _ => {
            let mut table_headers = vec![];

            let (headers, err) = parse_value(
                working_set,
                output.block[0].commands[0].parts[0],
                &SyntaxShape::List(Box::new(SyntaxShape::Any)),
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
                            error = error.or_else(|| {
                                Some(ParseError::MissingColumns(table_headers.len(), span))
                            })
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
                    span,
                    ty: Type::Table,
                },
                error,
            )
        }
    }
}

pub fn parse_block_expression(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Expression, Option<ParseError>) {
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
        error = error.or_else(|| {
            Some(ParseError::Unclosed(
                "}".into(),
                Span {
                    start: end,
                    end: end + 1,
                },
            ))
        });
    }

    let span = Span { start, end };

    let source = working_set.get_span_contents(span);

    let (output, err) = lex(source, start, &[], &[]);
    error = error.or(err);

    working_set.enter_scope();

    // Check to see if we have parameters
    let (signature, amt_to_skip): (Option<Box<Signature>>, usize) = match output.first() {
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

            let (signature, err) = parse_signature_helper(
                working_set,
                Span {
                    start: start_point,
                    end: end_point,
                },
            );
            error = error.or(err);

            (Some(signature), amt_to_skip)
        }
        _ => (None, 0),
    };

    let (output, err) = lite_parse(&output[amt_to_skip..]);
    error = error.or(err);

    let (mut output, err) = parse_block(working_set, &output, false);
    error = error.or(err);

    if let Some(signature) = signature {
        output.signature = signature;
    } else if let Some(last) = working_set.delta.scope.last() {
        if let Some(var_id) = last.get_var(b"$it") {
            let mut signature = Signature::new("");
            signature.required_positional.push(PositionalArg {
                var_id: Some(*var_id),
                name: "$it".into(),
                desc: String::new(),
                shape: SyntaxShape::Any,
            });
            output.signature = Box::new(signature);
        }
    }

    working_set.exit_scope();

    let block_id = working_set.add_block(output);

    (
        Expression {
            expr: Expr::Block(block_id),
            span,
            ty: Type::Block,
        },
        error,
    )
}

pub fn parse_value(
    working_set: &mut StateWorkingSet,
    span: Span,
    shape: &SyntaxShape,
) -> (Expression, Option<ParseError>) {
    let bytes = working_set.get_span_contents(span);

    // First, check the special-cases. These will likely represent specific values as expressions
    // and may fit a variety of shapes.
    //
    // We check variable first because immediately following we check for variables with cell paths
    // which might result in a value that fits other shapes (and require the variable to already be
    // declared)
    if shape == &SyntaxShape::Variable {
        return parse_variable_expr(working_set, span);
    } else if bytes.starts_with(b"$") {
        return parse_dollar_expr(working_set, span);
    } else if bytes.starts_with(b"(") {
        if let (expr, None) = parse_range(working_set, span) {
            return (expr, None);
        } else {
            return parse_full_column_path(working_set, None, span);
        }
    } else if bytes.starts_with(b"{") {
        if matches!(shape, SyntaxShape::Block) || matches!(shape, SyntaxShape::Any) {
            return parse_block_expression(working_set, span);
        } else {
            return (
                Expression::garbage(span),
                Some(ParseError::Expected("non-block value".into(), span)),
            );
        }
    } else if bytes.starts_with(b"[") {
        match shape {
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
        }
    }

    match shape {
        SyntaxShape::Number => parse_number(bytes, span),
        SyntaxShape::Int => parse_int(bytes, span),
        SyntaxShape::Range => parse_range(working_set, span),
        SyntaxShape::String | SyntaxShape::GlobPattern | SyntaxShape::FilePath => {
            parse_string(working_set, span)
        }
        SyntaxShape::Block => {
            if bytes.starts_with(b"{") {
                parse_block_expression(working_set, span)
            } else {
                (
                    Expression::garbage(span),
                    Some(ParseError::Expected("block".into(), span)),
                )
            }
        }
        SyntaxShape::Signature => {
            if bytes.starts_with(b"[") {
                parse_signature(working_set, span)
            } else {
                (
                    Expression::garbage(span),
                    Some(ParseError::Expected("signature".into(), span)),
                )
            }
        }
        SyntaxShape::List(elem) => {
            if bytes.starts_with(b"[") {
                parse_list_expression(working_set, span, elem)
            } else {
                (
                    Expression::garbage(span),
                    Some(ParseError::Expected("list".into(), span)),
                )
            }
        }
        SyntaxShape::Table => {
            if bytes.starts_with(b"[") {
                parse_table_expression(working_set, span)
            } else {
                (
                    Expression::garbage(span),
                    Some(ParseError::Expected("table".into(), span)),
                )
            }
        }
        SyntaxShape::Any => {
            if bytes.starts_with(b"[") {
                parse_value(working_set, span, &SyntaxShape::Table)
            } else {
                let shapes = [
                    SyntaxShape::Int,
                    SyntaxShape::Number,
                    SyntaxShape::Range,
                    SyntaxShape::Filesize,
                    SyntaxShape::Duration,
                    SyntaxShape::Block,
                    SyntaxShape::String,
                ];
                for shape in shapes.iter() {
                    if let (s, None) = parse_value(working_set, span, shape) {
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
        b"==" => Operator::Equal,
        b"!=" => Operator::NotEqual,
        b"<" => Operator::LessThan,
        b"<=" => Operator::LessThanOrEqual,
        b">" => Operator::GreaterThan,
        b">=" => Operator::GreaterThanOrEqual,
        b"=~" => Operator::Contains,
        b"!~" => Operator::NotContains,
        b"+" => Operator::Plus,
        b"-" => Operator::Minus,
        b"*" => Operator::Multiply,
        b"/" => Operator::Divide,
        b"in" => Operator::In,
        b"not-in" => Operator::NotIn,
        b"mod" => Operator::Modulo,
        b"&&" => Operator::And,
        b"||" => Operator::Or,
        b"**" => Operator::Pow,
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
            ty: Type::Unknown,
        },
        None,
    )
}

pub fn parse_math_expression(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    lhs_row_var_id: Option<VarId>,
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
    let (lhs, err) = parse_value(working_set, spans[0], &SyntaxShape::Any);
    error = error.or(err);
    idx += 1;

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

        let (rhs, err) = parse_value(working_set, spans[idx], &SyntaxShape::Any);
        error = error.or(err);

        if op_prec <= last_prec {
            while expr_stack.len() > 1 {
                // Collapse the right associated operations first
                // so that we can get back to a stack with a lower precedence
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
                    expand_to_cell_path(working_set, &mut lhs, row_var_id);
                }

                let (result_ty, err) = math_result_type(working_set, &mut lhs, &mut op, &mut rhs);
                error = error.or(err);

                let op_span = span(&[lhs.span, rhs.span]);
                expr_stack.push(Expression {
                    expr: Expr::BinaryOp(Box::new(lhs), Box::new(op), Box::new(rhs)),
                    span: op_span,
                    ty: result_ty,
                });
            }
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
            expand_to_cell_path(working_set, &mut lhs, row_var_id);
        }

        let (result_ty, err) = math_result_type(working_set, &mut lhs, &mut op, &mut rhs);
        error = error.or(err);

        let binary_op_span = span(&[lhs.span, rhs.span]);
        expr_stack.push(Expression {
            expr: Expr::BinaryOp(Box::new(lhs), Box::new(op), Box::new(rhs)),
            span: binary_op_span,
            ty: result_ty,
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
) -> (Expression, Option<ParseError>) {
    let bytes = working_set.get_span_contents(spans[0]);

    match bytes[0] {
        b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' | b'(' | b'{'
        | b'[' | b'$' | b'"' | b'\'' | b'-' => parse_math_expression(working_set, spans, None),
        _ => parse_call(working_set, spans, true),
    }
}

pub fn parse_variable(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> (Option<VarId>, Option<ParseError>) {
    let bytes = working_set.get_span_contents(span);

    if is_variable(bytes) {
        if let Some(var_id) = working_set.find_variable(bytes) {
            (Some(var_id), None)
        } else {
            (None, None)
        }
    } else {
        (None, Some(ParseError::Expected("variable".into(), span)))
    }
}

pub fn parse_def_predecl(working_set: &mut StateWorkingSet, spans: &[Span]) {
    let name = working_set.get_span_contents(spans[0]);

    if name == b"def" && spans.len() >= 4 {
        let (name_expr, ..) = parse_string(working_set, spans[1]);
        let name = name_expr.as_string();

        working_set.enter_scope();
        // FIXME: because parse_signature will update the scope with the variables it sees
        // we end up parsing the signature twice per def. The first time is during the predecl
        // so that we can see the types that are part of the signature, which we need for parsing.
        // The second time is when we actually parse the body itworking_set.
        // We can't reuse the first time because the variables that are created during parse_signature
        // are lost when we exit the scope below.
        let (sig, ..) = parse_signature(working_set, spans[2]);
        let signature = sig.as_signature();
        working_set.exit_scope();

        if let (Some(name), Some(mut signature)) = (name, signature) {
            signature.name = name;
            let decl = signature.predeclare();

            working_set.add_decl(decl);
        }
    }
}

pub fn parse_def(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> (Statement, Option<ParseError>) {
    let mut error = None;
    let name = working_set.get_span_contents(spans[0]);

    if name == b"def" && spans.len() >= 4 {
        //FIXME: don't use expect here
        let (name_expr, err) = parse_string(working_set, spans[1]);
        error = error.or(err);

        working_set.enter_scope();
        let (sig, err) = parse_signature(working_set, spans[2]);
        error = error.or(err);

        let (block, err) = parse_block_expression(working_set, spans[3]);
        error = error.or(err);
        working_set.exit_scope();

        if error.is_some() {
            return (
                Statement::Pipeline(Pipeline::from_vec(vec![garbage(span(spans))])),
                error,
            );
        }

        let name = name_expr.as_string();

        let signature = sig.as_signature();

        let block_id = block.as_block();

        match (name, signature, block_id) {
            (Some(name), Some(mut signature), Some(block_id)) => {
                let decl_id = working_set
                    .find_decl(name.as_bytes())
                    .expect("internal error: predeclaration failed to add definition");

                let declaration = working_set.get_decl_mut(decl_id);

                signature.name = name;

                *declaration = signature.into_block_command(block_id);

                let def_decl_id = working_set
                    .find_decl(b"def")
                    .expect("internal error: missing def command");

                let call = Box::new(Call {
                    head: spans[0],
                    decl_id: def_decl_id,
                    positional: vec![name_expr, sig, block],
                    named: vec![],
                });

                (
                    Statement::Pipeline(Pipeline::from_vec(vec![Expression {
                        expr: Expr::Call(call),
                        span: span(spans),
                        ty: Type::Unknown,
                    }])),
                    error,
                )
            }
            _ => (
                Statement::Pipeline(Pipeline::from_vec(vec![garbage(span(spans))])),
                error,
            ),
        }
    } else {
        (
            garbage_statement(spans),
            Some(ParseError::UnknownState(
                "definition unparseable. Expected structure: def <name> [] {}".into(),
                span(spans),
            )),
        )
    }
}

pub fn parse_alias(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> (Statement, Option<ParseError>) {
    let name = working_set.get_span_contents(spans[0]);

    if name == b"alias" {
        if let Some(err) = check_name(working_set, spans) {
            return (
                Statement::Pipeline(Pipeline::from_vec(vec![garbage(span(spans))])),
                Some(err),
            );
        }

        if let Some(decl_id) = working_set.find_decl(b"alias") {
            let (call, call_span, _) =
                parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

            if spans.len() >= 4 {
                let alias_name = working_set.get_span_contents(spans[1]);

                let alias_name = if alias_name.starts_with(b"\"")
                    && alias_name.ends_with(b"\"")
                    && alias_name.len() > 1
                {
                    alias_name[1..(alias_name.len() - 1)].to_vec()
                } else {
                    alias_name.to_vec()
                };
                let _equals = working_set.get_span_contents(spans[2]);

                let replacement = spans[3..].to_vec();

                //println!("{:?} {:?}", alias_name, replacement);

                working_set.add_alias(alias_name, replacement);
            }

            return (
                Statement::Pipeline(Pipeline::from_vec(vec![Expression {
                    expr: Expr::Call(call),
                    span: call_span,
                    ty: Type::Unknown,
                }])),
                None,
            );
        }
    }

    (
        garbage_statement(spans),
        Some(ParseError::UnknownState(
            "internal error: alias statement unparseable".into(),
            span(spans),
        )),
    )
}

pub fn parse_let(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> (Statement, Option<ParseError>) {
    let name = working_set.get_span_contents(spans[0]);

    if name == b"let" {
        if let Some(err) = check_name(working_set, spans) {
            return (
                Statement::Pipeline(Pipeline::from_vec(vec![garbage(span(spans))])),
                Some(err),
            );
        }

        if let Some(decl_id) = working_set.find_decl(b"let") {
            let (call, call_span, err) =
                parse_internal_call(working_set, spans[0], &spans[1..], decl_id);

            // Update the variable to the known type if we can.
            if err.is_none() {
                let var_id = call.positional[0]
                    .as_var()
                    .expect("internal error: expected variable");
                let rhs_type = call.positional[1].ty.clone();

                working_set.set_variable_type(var_id, rhs_type);
            }

            return (
                Statement::Pipeline(Pipeline::from_vec(vec![Expression {
                    expr: Expr::Call(call),
                    span: call_span,
                    ty: Type::Unknown,
                }])),
                err,
            );
        }
    }
    (
        garbage_statement(spans),
        Some(ParseError::UnknownState(
            "internal error: let statement unparseable".into(),
            span(spans),
        )),
    )
}

pub fn parse_statement(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> (Statement, Option<ParseError>) {
    // FIXME: improve errors by checking keyword first
    if let (decl, None) = parse_def(working_set, spans) {
        (decl, None)
    } else if let (stmt, None) = parse_let(working_set, spans) {
        (stmt, None)
    } else if let (stmt, None) = parse_alias(working_set, spans) {
        (stmt, None)
    } else {
        let (expr, err) = parse_expression(working_set, spans);
        (Statement::Pipeline(Pipeline::from_vec(vec![expr])), err)
    }
}

pub fn parse_block(
    working_set: &mut StateWorkingSet,
    lite_block: &LiteBlock,
    scoped: bool,
) -> (Block, Option<ParseError>) {
    if scoped {
        working_set.enter_scope();
    }

    // Pre-declare any definition so that definitions
    // that share the same block can see each other
    for pipeline in &lite_block.block {
        if pipeline.commands.len() == 1 {
            parse_def_predecl(working_set, &pipeline.commands[0].parts);
        }
    }

    let mut error = None;

    let block: Block = lite_block
        .block
        .iter()
        .map(|pipeline| {
            if pipeline.commands.len() > 1 {
                let output = pipeline
                    .commands
                    .iter()
                    .map(|command| {
                        let (expr, err) = parse_expression(working_set, &command.parts);

                        if error.is_none() {
                            error = err;
                        }

                        expr
                    })
                    .collect::<Vec<Expression>>();

                Statement::Pipeline(Pipeline {
                    expressions: output,
                })
            } else {
                let (stmt, err) = parse_statement(working_set, &pipeline.commands[0].parts);

                if error.is_none() {
                    error = err;
                }

                stmt
            }
        })
        .into();

    if scoped {
        working_set.exit_scope();
    }

    (block, error)
}

// Parses a vector of u8 to create an AST Block. If a file name is given, then
// the name is stored in the working set. When parsing a source without a file
// name, the source of bytes is stored as "source"
pub fn parse(
    working_set: &mut StateWorkingSet,
    fname: Option<&str>,
    contents: &[u8],
    scoped: bool,
) -> (Block, Option<ParseError>) {
    let mut error = None;

    let span_offset = working_set.next_span_start();

    let name = match fname {
        Some(fname) => fname.to_string(),
        None => "source".to_string(),
    };

    working_set.add_file(name, contents);

    let (output, err) = lex(contents, span_offset, &[], &[]);
    error = error.or(err);

    let (output, err) = lite_parse(&output);
    error = error.or(err);

    let (output, err) = parse_block(working_set, &output, scoped);
    error = error.or(err);

    (output, error)
}
