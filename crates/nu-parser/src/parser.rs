use crate::{
    lex::{lex, lex_signature},
    lite_parser::{lite_parse, LiteCommand, LitePipeline, LiteRedirection, LiteRedirectionTarget},
    parse_keywords::*,
    parse_patterns::parse_pattern,
    parse_shape_specs::{parse_shape_name, parse_type, ShapeDescriptorUse},
    type_check::{self, math_result_type, type_compatible},
    Token, TokenContents,
};
use itertools::Itertools;
use log::trace;
use nu_engine::DIR_VAR_PARSER_INFO;
use nu_protocol::{
    ast::*, engine::StateWorkingSet, eval_const::eval_constant, BlockId, DidYouMean, Flag,
    ParseError, PositionalArg, Signature, Span, Spanned, SyntaxShape, Type, VarId, ENV_VARIABLE_ID,
    IN_VARIABLE_ID,
};
use std::{
    collections::{HashMap, HashSet},
    num::ParseIntError,
    str,
    sync::Arc,
};

pub fn garbage(span: Span) -> Expression {
    Expression::garbage(span)
}

pub fn garbage_pipeline(spans: &[Span]) -> Pipeline {
    Pipeline::from_vec(vec![garbage(Span::concat(spans))])
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

pub fn is_math_expression_like(working_set: &mut StateWorkingSet, span: Span) -> bool {
    let bytes = working_set.get_span_contents(span);
    if bytes.is_empty() {
        return false;
    }

    if bytes == b"true"
        || bytes == b"false"
        || bytes == b"null"
        || bytes == b"not"
        || bytes == b"if"
        || bytes == b"match"
    {
        return true;
    }

    let b = bytes[0];

    // check for raw string
    if bytes.starts_with(b"r#") {
        return true;
    }

    if b == b'(' || b == b'{' || b == b'[' || b == b'$' || b == b'"' || b == b'\'' || b == b'-' {
        return true;
    }

    let starting_error_count = working_set.parse_errors.len();

    // Number
    parse_number(working_set, span);
    if working_set.parse_errors.len() == starting_error_count {
        return true;
    }
    working_set.parse_errors.truncate(starting_error_count);

    // Filesize
    parse_filesize(working_set, span);
    if working_set.parse_errors.len() == starting_error_count {
        return true;
    }
    working_set.parse_errors.truncate(starting_error_count);

    parse_duration(working_set, span);
    if working_set.parse_errors.len() == starting_error_count {
        return true;
    }
    working_set.parse_errors.truncate(starting_error_count);

    parse_datetime(working_set, span);
    if working_set.parse_errors.len() == starting_error_count {
        return true;
    }
    working_set.parse_errors.truncate(starting_error_count);

    parse_binary(working_set, span);
    if working_set.parse_errors.len() == starting_error_count {
        return true;
    }
    working_set.parse_errors.truncate(starting_error_count);

    parse_range(working_set, span);

    if working_set.parse_errors.len() == starting_error_count {
        return true;
    }
    working_set.parse_errors.truncate(starting_error_count);

    false
}

fn is_identifier(bytes: &[u8]) -> bool {
    bytes.iter().all(|x| is_identifier_byte(*x))
}

pub fn is_variable(bytes: &[u8]) -> bool {
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

pub(crate) fn check_call(
    working_set: &mut StateWorkingSet,
    command: Span,
    sig: &Signature,
    call: &Call,
) {
    // Allow the call to pass if they pass in the help flag
    if call.named_iter().any(|(n, _, _)| n.item == "help") {
        return;
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
                    working_set.error(ParseError::MissingPositional(
                        argument.name.clone(),
                        Span::new(last.span.end, last.span.end),
                        sig.call_signature(),
                    ));
                    return;
                } else {
                    working_set.error(ParseError::MissingPositional(
                        argument.name.clone(),
                        Span::new(command.end, command.end),
                        sig.call_signature(),
                    ));
                    return;
                }
            }
        }

        let missing = &sig.required_positional[call.positional_len()];
        if let Some(last) = call.positional_iter().last() {
            working_set.error(ParseError::MissingPositional(
                missing.name.clone(),
                Span::new(last.span.end, last.span.end),
                sig.call_signature(),
            ))
        } else {
            working_set.error(ParseError::MissingPositional(
                missing.name.clone(),
                Span::new(command.end, command.end),
                sig.call_signature(),
            ))
        }
    } else {
        for req_flag in sig.named.iter().filter(|x| x.required) {
            if call.named_iter().all(|(n, _, _)| n.item != req_flag.long) {
                working_set.error(ParseError::MissingRequiredFlag(
                    req_flag.long.clone(),
                    command,
                ));
            }
        }
    }
}

fn parse_external_arg(working_set: &mut StateWorkingSet, span: Span) -> ExternalArgument {
    let contents = working_set.get_span_contents(span);

    if contents.starts_with(b"$") || contents.starts_with(b"(") {
        ExternalArgument::Regular(parse_dollar_expr(working_set, span))
    } else if contents.starts_with(b"[") {
        ExternalArgument::Regular(parse_list_expression(working_set, span, &SyntaxShape::Any))
    } else if contents.starts_with(b"r#") {
        ExternalArgument::Regular(parse_raw_string(working_set, span))
    } else if contents.len() > 3
        && contents.starts_with(b"...")
        && (contents[3] == b'$' || contents[3] == b'[' || contents[3] == b'(')
    {
        ExternalArgument::Spread(parse_value(
            working_set,
            Span::new(span.start + 3, span.end),
            &SyntaxShape::List(Box::new(SyntaxShape::Any)),
        ))
    } else {
        // Eval stage trims the quotes, so we don't have to do the same thing when parsing.
        let contents = if contents.starts_with(b"\"") {
            let (contents, err) = unescape_string(contents, span);
            if let Some(err) = err {
                working_set.error(err)
            }
            String::from_utf8_lossy(&contents).to_string()
        } else {
            String::from_utf8_lossy(contents).to_string()
        };

        ExternalArgument::Regular(Expression {
            expr: Expr::String(contents),
            span,
            ty: Type::String,
            custom_completion: None,
        })
    }
}

pub fn parse_external_call(working_set: &mut StateWorkingSet, spans: &[Span]) -> Expression {
    trace!("parse external");

    let head_contents = working_set.get_span_contents(spans[0]);

    let head_span = if head_contents.starts_with(b"^") {
        Span::new(spans[0].start + 1, spans[0].end)
    } else {
        spans[0]
    };

    let head_contents = working_set.get_span_contents(head_span).to_vec();

    let head = if head_contents.starts_with(b"$") || head_contents.starts_with(b"(") {
        // the expression is inside external_call, so it's a subexpression
        let arg = parse_expression(working_set, &[head_span]);
        Box::new(arg)
    } else {
        let (contents, err) = unescape_unquote_string(&head_contents, head_span);
        if let Some(err) = err {
            working_set.error(err)
        }

        Box::new(Expression {
            expr: Expr::String(contents),
            span: head_span,
            ty: Type::String,
            custom_completion: None,
        })
    };

    let args = spans[1..]
        .iter()
        .map(|&span| parse_external_arg(working_set, span))
        .collect();

    Expression {
        expr: Expr::ExternalCall(head, args),
        span: Span::concat(spans),
        ty: Type::Any,
        custom_completion: None,
    }
}

fn ensure_flag_arg_type(
    working_set: &mut StateWorkingSet,
    arg_name: String,
    arg: Expression,
    arg_shape: &SyntaxShape,
    long_name_span: Span,
) -> (Spanned<String>, Expression) {
    if !type_compatible(&arg.ty, &arg_shape.to_type()) {
        working_set.error(ParseError::TypeMismatch(
            arg_shape.to_type(),
            arg.ty,
            arg.span,
        ));
        (
            Spanned {
                item: arg_name,
                span: long_name_span,
            },
            Expression::garbage(arg.span),
        )
    } else {
        (
            Spanned {
                item: arg_name,
                span: long_name_span,
            },
            arg,
        )
    }
}

fn parse_long_flag(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    spans_idx: &mut usize,
    sig: &Signature,
) -> (Option<Spanned<String>>, Option<Expression>) {
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

                        let arg = parse_value(working_set, span, arg_shape);
                        let (arg_name, val_expression) = ensure_flag_arg_type(
                            working_set,
                            long_name,
                            arg,
                            arg_shape,
                            Span::new(arg_span.start, arg_span.start + long_name_len + 2),
                        );
                        (Some(arg_name), Some(val_expression))
                    } else if let Some(arg) = spans.get(*spans_idx + 1) {
                        let arg = parse_value(working_set, *arg, arg_shape);

                        *spans_idx += 1;
                        let (arg_name, val_expression) =
                            ensure_flag_arg_type(working_set, long_name, arg, arg_shape, arg_span);
                        (Some(arg_name), Some(val_expression))
                    } else {
                        working_set.error(ParseError::MissingFlagParam(
                            arg_shape.to_string(),
                            arg_span,
                        ));
                        (
                            Some(Spanned {
                                item: long_name,
                                span: arg_span,
                            }),
                            None,
                        )
                    }
                } else {
                    // A flag with no argument
                    // It can also takes a boolean value like --x=true
                    if split.len() > 1 {
                        // and we also have the argument
                        let long_name_len = long_name.len();
                        let mut span = arg_span;
                        span.start += long_name_len + 3; //offset by long flag and '='

                        let arg = parse_value(working_set, span, &SyntaxShape::Boolean);

                        let (arg_name, val_expression) = ensure_flag_arg_type(
                            working_set,
                            long_name,
                            arg,
                            &SyntaxShape::Boolean,
                            Span::new(arg_span.start, arg_span.start + long_name_len + 2),
                        );
                        (Some(arg_name), Some(val_expression))
                    } else {
                        (
                            Some(Spanned {
                                item: long_name,
                                span: arg_span,
                            }),
                            None,
                        )
                    }
                }
            } else {
                working_set.error(ParseError::UnknownFlag(
                    sig.name.clone(),
                    long_name.clone(),
                    arg_span,
                    sig.clone().formatted_flags(),
                ));
                (
                    Some(Spanned {
                        item: long_name.clone(),
                        span: arg_span,
                    }),
                    None,
                )
            }
        } else {
            working_set.error(ParseError::NonUtf8(arg_span));
            (
                Some(Spanned {
                    item: "--".into(),
                    span: arg_span,
                }),
                None,
            )
        }
    } else {
        (None, None)
    }
}

fn parse_short_flags(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    spans_idx: &mut usize,
    positional_idx: usize,
    sig: &Signature,
) -> Option<Vec<Flag>> {
    let arg_span = spans[*spans_idx];

    let arg_contents = working_set.get_span_contents(arg_span);

    if let Ok(arg_contents_uft8_ref) = str::from_utf8(arg_contents) {
        if arg_contents_uft8_ref.starts_with('-') && arg_contents_uft8_ref.len() > 1 {
            let short_flags = &arg_contents_uft8_ref[1..];
            let num_chars = short_flags.chars().count();
            let mut found_short_flags = vec![];
            let mut unmatched_short_flags = vec![];
            for (offset, short_flag) in short_flags.char_indices() {
                let short_flag_span = Span::new(
                    arg_span.start + 1 + offset,
                    arg_span.start + 1 + offset + short_flag.len_utf8(),
                );
                if let Some(flag) = sig.get_short_flag(short_flag) {
                    // Allow args in short flag batches as long as it is the last flag.
                    if flag.arg.is_some() && offset < num_chars - 1 {
                        working_set
                            .error(ParseError::OnlyLastFlagInBatchCanTakeArg(short_flag_span));
                        break;
                    }
                    found_short_flags.push(flag);
                } else {
                    unmatched_short_flags.push(short_flag_span);
                }
            }

            if found_short_flags.is_empty()
                // check to see if we have a negative number
                && matches!(
                    sig.get_positional(positional_idx),
                    Some(PositionalArg {
                        shape: SyntaxShape::Int | SyntaxShape::Number | SyntaxShape::Float,
                        ..
                    })
                )
                && String::from_utf8_lossy(working_set.get_span_contents(arg_span))
                    .parse::<f64>()
                    .is_ok()
            {
                return None;
            } else if let Some(first) = unmatched_short_flags.first() {
                let contents = working_set.get_span_contents(*first);
                working_set.error(ParseError::UnknownFlag(
                    sig.name.clone(),
                    format!("-{}", String::from_utf8_lossy(contents)),
                    *first,
                    sig.clone().formatted_flags(),
                ));
            }

            Some(found_short_flags)
        } else {
            None
        }
    } else {
        working_set.error(ParseError::NonUtf8(arg_span));
        None
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
            for (span_idx, &span) in spans.iter().enumerate().skip(spans_idx) {
                let contents = working_set.get_span_contents(span);

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
) -> Expression {
    trace!("parse multispan value");
    match shape {
        SyntaxShape::VarWithOptType => {
            trace!("parsing: var with opt type");

            parse_var_with_opt_type(working_set, spans, spans_idx, false).0
        }
        SyntaxShape::RowCondition => {
            trace!("parsing: row condition");
            let arg = parse_row_condition(working_set, &spans[*spans_idx..]);
            *spans_idx = spans.len() - 1;

            arg
        }
        SyntaxShape::MathExpression => {
            trace!("parsing: math expression");

            let arg = parse_math_expression(working_set, &spans[*spans_idx..], None);
            *spans_idx = spans.len() - 1;

            arg
        }
        SyntaxShape::OneOf(shapes) => {
            // handle for `if` command.
            //let block_then_exp = shapes.as_slice() == [SyntaxShape::Block, SyntaxShape::Expression];
            for shape in shapes.iter() {
                let starting_error_count = working_set.parse_errors.len();
                let s = parse_multispan_value(working_set, spans, spans_idx, shape);

                if starting_error_count == working_set.parse_errors.len() {
                    return s;
                } else if let Some(
                    ParseError::Expected(..) | ParseError::ExpectedWithStringMsg(..),
                ) = working_set.parse_errors.last()
                {
                    working_set.parse_errors.truncate(starting_error_count);
                    continue;
                }
                // `if` is parsing block first and then expression.
                // when we're writing something like `else if $a`, parsing as a
                // block will result to error(because it's not a block)
                //
                // If parse as a expression also failed, user is more likely concerned
                // about expression failure rather than "expect block failure"".

                // FIXME FIXME FIXME
                // if block_then_exp {
                //     match &err {
                //         Some(ParseError::Expected(expected, _)) => {
                //             if expected.starts_with("block") {
                //                 err = e
                //             }
                //         }
                //         _ => err = err.or(e),
                //     }
                // } else {
                //     err = err.or(e)
                // }
            }
            let span = spans[*spans_idx];

            if working_set.parse_errors.is_empty() {
                working_set.error(ParseError::ExpectedWithStringMsg(
                    format!("one of a list of accepted shapes: {shapes:?}"),
                    span,
                ));
            }

            Expression::garbage(span)
        }
        SyntaxShape::Expression => {
            trace!("parsing: expression");

            // is it subexpression?
            // Not sure, but let's make it not, so the behavior is the same as previous version of nushell.
            let arg = parse_expression(working_set, &spans[*spans_idx..]);
            *spans_idx = spans.len() - 1;

            arg
        }
        SyntaxShape::Signature => {
            trace!("parsing: signature");

            let sig = parse_full_signature(working_set, &spans[*spans_idx..]);
            *spans_idx = spans.len() - 1;

            sig
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
                working_set.error(ParseError::ExpectedKeyword(
                    String::from_utf8_lossy(keyword).into(),
                    arg_span,
                ))
            }

            *spans_idx += 1;
            if *spans_idx >= spans.len() {
                working_set.error(ParseError::KeywordMissingArgument(
                    arg.to_string(),
                    String::from_utf8_lossy(keyword).into(),
                    Span::new(spans[*spans_idx - 1].end, spans[*spans_idx - 1].end),
                ));
                let keyword = Keyword {
                    keyword: keyword.as_slice().into(),
                    span: spans[*spans_idx - 1],
                    expr: Expression::garbage(arg_span),
                };
                return Expression {
                    expr: Expr::Keyword(Box::new(keyword)),
                    span: arg_span,
                    ty: Type::Any,
                    custom_completion: None,
                };
            }

            let keyword = Keyword {
                keyword: keyword.as_slice().into(),
                span: spans[*spans_idx - 1],
                expr: parse_multispan_value(working_set, spans, spans_idx, arg),
            };

            Expression {
                ty: keyword.expr.ty.clone(),
                expr: Expr::Keyword(Box::new(keyword)),
                span: arg_span,
                custom_completion: None,
            }
        }
        _ => {
            // All other cases are single-span values
            let arg_span = spans[*spans_idx];

            parse_value(working_set, arg_span, shape)
        }
    }
}

pub struct ParsedInternalCall {
    pub call: Box<Call>,
    pub output: Type,
}

fn attach_parser_info_builtin(working_set: &StateWorkingSet, name: &str, call: &mut Call) {
    match name {
        "use" | "overlay use" | "source-env" | "nu-check" => {
            if let Some(var_id) = find_dirs_var(working_set, LIB_DIRS_VAR) {
                call.set_parser_info(
                    DIR_VAR_PARSER_INFO.to_owned(),
                    Expression {
                        expr: Expr::Var(var_id),
                        span: call.head,
                        ty: Type::Any,
                        custom_completion: None,
                    },
                );
            }
        }
        _ => {}
    }
}

pub fn parse_internal_call(
    working_set: &mut StateWorkingSet,
    command_span: Span,
    spans: &[Span],
    decl_id: usize,
) -> ParsedInternalCall {
    trace!("parsing: internal call (decl id: {})", decl_id);

    let mut call = Call::new(command_span);
    call.decl_id = decl_id;
    call.head = command_span;

    let decl = working_set.get_decl(decl_id);
    let signature = decl.signature();
    let output = signature.get_output_type();

    if decl.is_builtin() {
        attach_parser_info_builtin(working_set, decl.name(), &mut call);
    }

    // The index into the positional parameter in the definition
    let mut positional_idx = 0;

    // The index into the spans of argument data given to parse
    // Starting at the first argument
    let mut spans_idx = 0;

    if let Some(alias) = decl.as_alias() {
        if let Expression {
            expr: Expr::Call(wrapped_call),
            ..
        } = &alias.wrapped_call
        {
            // Replace this command's call with the aliased call, but keep the alias name
            call = *wrapped_call.clone();
            call.head = command_span;
            // Skip positionals passed to aliased call
            positional_idx = call.positional_len();
        } else {
            working_set.error(ParseError::UnknownState(
                "Alias does not point to internal call.".to_string(),
                command_span,
            ));
            return ParsedInternalCall {
                call: Box::new(call),
                output: Type::Any,
            };
        }
    }

    if signature.creates_scope {
        working_set.enter_scope();
    }

    while spans_idx < spans.len() {
        let arg_span = spans[spans_idx];

        let starting_error_count = working_set.parse_errors.len();
        // Check if we're on a long flag, if so, parse
        let (long_name, arg) = parse_long_flag(working_set, spans, &mut spans_idx, &signature);

        if let Some(long_name) = long_name {
            // We found a long flag, like --bar
            if working_set.parse_errors[starting_error_count..]
                .iter()
                .any(|x| matches!(x, ParseError::UnknownFlag(_, _, _, _)))
                && signature.allows_unknown_args
            {
                working_set.parse_errors.truncate(starting_error_count);
                let arg = parse_value(working_set, arg_span, &SyntaxShape::Any);

                call.add_unknown(arg);
            } else {
                call.add_named((long_name, None, arg));
            }

            spans_idx += 1;
            continue;
        }

        let starting_error_count = working_set.parse_errors.len();

        // Check if we're on a short flag or group of short flags, if so, parse
        let short_flags = parse_short_flags(
            working_set,
            spans,
            &mut spans_idx,
            positional_idx,
            &signature,
        );

        if let Some(mut short_flags) = short_flags {
            if short_flags.is_empty() {
                // workaround for completions (PR #6067)
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

            if working_set.parse_errors[starting_error_count..]
                .iter()
                .any(|x| matches!(x, ParseError::UnknownFlag(_, _, _, _)))
                && signature.allows_unknown_args
            {
                working_set.parse_errors.truncate(starting_error_count);
                let arg = parse_value(working_set, arg_span, &SyntaxShape::Any);

                call.add_unknown(arg);
            } else {
                for flag in short_flags {
                    if let Some(arg_shape) = flag.arg {
                        if let Some(arg) = spans.get(spans_idx + 1) {
                            let arg = parse_value(working_set, *arg, &arg_shape);

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
                            working_set.error(ParseError::MissingFlagParam(
                                arg_shape.to_string(),
                                arg_span,
                            ))
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
            }

            spans_idx += 1;
            continue;
        }

        {
            let contents = working_set.get_span_contents(spans[spans_idx]);

            if contents.len() > 3
                && contents.starts_with(b"...")
                && (contents[3] == b'$' || contents[3] == b'[' || contents[3] == b'(')
            {
                if signature.rest_positional.is_none() && !signature.allows_unknown_args {
                    working_set.error(ParseError::UnexpectedSpreadArg(
                        signature.call_signature(),
                        arg_span,
                    ));
                    call.add_positional(Expression::garbage(arg_span));
                } else if positional_idx < signature.required_positional.len() {
                    working_set.error(ParseError::MissingPositional(
                        signature.required_positional[positional_idx].name.clone(),
                        Span::new(spans[spans_idx].start, spans[spans_idx].start),
                        signature.call_signature(),
                    ));
                    call.add_positional(Expression::garbage(arg_span));
                } else {
                    let rest_shape = match &signature.rest_positional {
                        Some(arg) => arg.shape.clone(),
                        None => SyntaxShape::Any,
                    };
                    // Parse list of arguments to be spread
                    let args = parse_value(
                        working_set,
                        Span::new(arg_span.start + 3, arg_span.end),
                        &SyntaxShape::List(Box::new(rest_shape)),
                    );

                    call.add_spread(args);
                    // Let the parser know that it's parsing rest arguments now
                    positional_idx =
                        signature.required_positional.len() + signature.optional_positional.len();
                }

                spans_idx += 1;
                continue;
            }
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
                working_set.error(ParseError::MissingPositional(
                    positional.name.clone(),
                    Span::new(spans[spans_idx].end, spans[spans_idx].end),
                    signature.call_signature(),
                ));
                positional_idx += 1;
                continue;
            }

            let arg = parse_multispan_value(
                working_set,
                &spans[..end],
                &mut spans_idx,
                &positional.shape,
            );

            let arg = if !type_compatible(&positional.shape.to_type(), &arg.ty) {
                working_set.error(ParseError::TypeMismatch(
                    positional.shape.to_type(),
                    arg.ty,
                    arg.span,
                ));
                Expression::garbage(arg.span)
            } else {
                arg
            };
            call.add_positional(arg);
            positional_idx += 1;
        } else if signature.allows_unknown_args {
            let arg = parse_value(working_set, arg_span, &SyntaxShape::Any);

            call.add_unknown(arg);
        } else {
            call.add_positional(Expression::garbage(arg_span));
            working_set.error(ParseError::ExtraPositional(
                signature.call_signature(),
                arg_span,
            ))
        }

        spans_idx += 1;
    }

    check_call(working_set, command_span, &signature, &call);

    if signature.creates_scope {
        working_set.exit_scope();
    }

    ParsedInternalCall {
        call: Box::new(call),
        output,
    }
}

pub fn parse_call(working_set: &mut StateWorkingSet, spans: &[Span], head: Span) -> Expression {
    trace!("parsing: call");

    if spans.is_empty() {
        working_set.error(ParseError::UnknownState(
            "Encountered command with zero spans".into(),
            Span::concat(spans),
        ));
        return garbage(head);
    }

    let mut pos = 0;
    let cmd_start = pos;
    let mut name_spans = vec![];
    let mut name = vec![];

    for word_span in spans[cmd_start..].iter() {
        // Find the longest group of words that could form a command

        name_spans.push(*word_span);

        let name_part = working_set.get_span_contents(*word_span);
        if name.is_empty() {
            name.extend(name_part);
        } else {
            name.push(b' ');
            name.extend(name_part);
        }

        pos += 1;
    }

    let mut maybe_decl_id = working_set.find_decl(&name);

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
        maybe_decl_id = working_set.find_decl(&name);
    }

    if let Some(decl_id) = maybe_decl_id {
        // Before the internal parsing we check if there is no let or alias declarations
        // that are missing their name, e.g.: let = 1 or alias = 2
        if spans.len() > 1 {
            let test_equal = working_set.get_span_contents(spans[1]);

            if test_equal == [b'='] {
                trace!("incomplete statement");

                working_set.error(ParseError::UnknownState(
                    "Incomplete statement".into(),
                    Span::concat(spans),
                ));
                return garbage(Span::concat(spans));
            }
        }

        // TODO: Try to remove the clone
        let decl = working_set.get_decl(decl_id);

        let parsed_call = if let Some(alias) = decl.as_alias() {
            if let Expression {
                expr: Expr::ExternalCall(head, args),
                span: _,
                ty,
                custom_completion,
            } = &alias.clone().wrapped_call
            {
                trace!("parsing: alias of external call");

                let mut head = head.clone();
                head.span = spans[0]; // replacing the spans preserves syntax highlighting

                let mut final_args = args.clone().into_vec();
                for arg_span in &spans[1..] {
                    let arg = parse_external_arg(working_set, *arg_span);
                    final_args.push(arg);
                }

                return Expression {
                    expr: Expr::ExternalCall(head, final_args.into()),
                    span: Span::concat(spans),
                    ty: ty.clone(),
                    custom_completion: *custom_completion,
                };
            } else {
                trace!("parsing: alias of internal call");
                parse_internal_call(
                    working_set,
                    Span::concat(&spans[cmd_start..pos]),
                    &spans[pos..],
                    decl_id,
                )
            }
        } else {
            trace!("parsing: internal call");
            parse_internal_call(
                working_set,
                Span::concat(&spans[cmd_start..pos]),
                &spans[pos..],
                decl_id,
            )
        };

        Expression {
            expr: Expr::Call(parsed_call.call),
            span: Span::concat(spans),
            ty: parsed_call.output,
            custom_completion: None,
        }
    } else {
        // We might be parsing left-unbounded range ("..10")
        let bytes = working_set.get_span_contents(spans[0]);
        trace!("parsing: range {:?} ", bytes);
        if let (Some(b'.'), Some(b'.')) = (bytes.first(), bytes.get(1)) {
            trace!("-- found leading range indicator");
            let starting_error_count = working_set.parse_errors.len();

            let range_expr = parse_range(working_set, spans[0]);
            if working_set.parse_errors.len() == starting_error_count {
                trace!("-- successfully parsed range");
                return range_expr;
            }
            working_set.parse_errors.truncate(starting_error_count);
        }
        trace!("parsing: external call");

        // Otherwise, try external command
        parse_external_call(working_set, spans)
    }
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
        if let Ok(num) = i64::from_str_radix(token, radix) {
            Expression {
                expr: Expr::Int(num),
                span,
                ty: Type::Int,
                custom_completion: None,
            }
        } else {
            working_set.error(ParseError::InvalidLiteral(
                format!("invalid digits for radix {}", radix),
                "int".into(),
                span,
            ));

            garbage(span)
        }
    }

    let token = strip_underscores(token);

    if token.is_empty() {
        working_set.error(ParseError::Expected("int", span));
        return garbage(span);
    }

    if let Some(num) = token.strip_prefix("0b") {
        extract_int(working_set, num, span, 2)
    } else if let Some(num) = token.strip_prefix("0o") {
        extract_int(working_set, num, span, 8)
    } else if let Some(num) = token.strip_prefix("0x") {
        extract_int(working_set, num, span, 16)
    } else if let Ok(num) = token.parse::<i64>() {
        Expression {
            expr: Expr::Int(num),
            span,
            ty: Type::Int,
            custom_completion: None,
        }
    } else {
        working_set.error(ParseError::Expected("int", span));
        garbage(span)
    }
}

pub fn parse_float(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let token = working_set.get_span_contents(span);
    let token = strip_underscores(token);

    if let Ok(x) = token.parse::<f64>() {
        Expression {
            expr: Expr::Float(x),
            span,
            ty: Type::Float,
            custom_completion: None,
        }
    } else {
        working_set.error(ParseError::Expected("float", span));

        garbage(span)
    }
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

pub fn parse_range(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: range");

    // Range follows the following syntax: [<from>][<next_operator><next>]<range_operator>[<to>]
    //   where <next_operator> is ".."
    //   and  <range_operator> is "..", "..=" or "..<"
    //   and one of the <from> or <to> bounds must be present (just '..' is not allowed since it
    //     looks like parent directory)
    //bugbug range cannot be [..] because that looks like parent directory

    let contents = working_set.get_span_contents(span);

    let token = if let Ok(s) = String::from_utf8(contents.into()) {
        s
    } else {
        working_set.error(ParseError::NonUtf8(span));
        return garbage(span);
    };

    if !token.contains("..") {
        working_set.error(ParseError::Expected("at least one range bound set", span));
        return garbage(span);
    }

    // First, figure out what exact operators are used and determine their positions
    let dotdot_pos: Vec<_> = token.match_indices("..").map(|(pos, _)| pos).collect();

    let (next_op_pos, range_op_pos) = match dotdot_pos.len() {
        1 => (None, dotdot_pos[0]),
        2 => (Some(dotdot_pos[0]), dotdot_pos[1]),
        _ => {
            working_set.error(ParseError::Expected(
                "one range operator ('..' or '..<') and optionally one next operator ('..')",
                span,
            ));
            return garbage(span);
        }
    };
    // Avoid calling sub-parsers on unmatched parens, to prevent quadratic time on things like ((((1..2))))
    // No need to call the expensive parse_value on "((((1"
    if dotdot_pos[0] > 0 {
        let (_tokens, err) = lex(
            &contents[..dotdot_pos[0]],
            span.start,
            &[],
            &[b'.', b'?'],
            true,
        );
        if let Some(_err) = err {
            working_set.error(ParseError::Expected("Valid expression before ..", span));
            return garbage(span);
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
            return garbage(span);
        }
    } else {
        let op_str = if token.contains("..=") { "..=" } else { ".." };
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
        Some(parse_value(working_set, from_span, &SyntaxShape::Number))
    };

    let to = if token.ends_with(range_op_str) {
        None
    } else {
        let to_span = Span::new(range_op_span.end, span.end);
        Some(parse_value(working_set, to_span, &SyntaxShape::Number))
    };

    trace!("-- from: {:?} to: {:?}", from, to);

    if let (None, None) = (&from, &to) {
        working_set.error(ParseError::Expected("at least one range bound set", span));
        return garbage(span);
    }

    let (next, next_op_span) = if let Some(pos) = next_op_pos {
        let next_op_span = Span::new(span.start + pos, span.start + pos + "..".len());
        let next_span = Span::new(next_op_span.end, range_op_span.start);

        (
            Some(parse_value(working_set, next_span, &SyntaxShape::Number)),
            next_op_span,
        )
    } else {
        (None, span)
    };

    let operator = RangeOperator {
        inclusion,
        span: range_op_span,
        next_op_span,
    };

    let range = Range {
        from,
        next,
        to,
        operator,
    };

    Expression {
        expr: Expr::Range(Box::new(range)),
        span,
        ty: Type::Range,
        custom_completion: None,
    }
}

pub(crate) fn parse_dollar_expr(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: dollar expression");
    let contents = working_set.get_span_contents(span);

    if contents.starts_with(b"$\"") || contents.starts_with(b"$'") {
        parse_string_interpolation(working_set, span)
    } else if contents.starts_with(b"$.") {
        parse_simple_cell_path(working_set, Span::new(span.start + 2, span.end))
    } else {
        let starting_error_count = working_set.parse_errors.len();

        let expr = parse_range(working_set, span);
        if starting_error_count == working_set.parse_errors.len() {
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
        return garbage(span);
    };
    let expect_postfix_sharp_cnt = prefix_sharp_cnt;
    // check the length of whole raw string.
    // the whole raw string should contains at least
    // 1(r) + prefix_sharp_cnt + 1(') + 1(') + postfix_sharp characters
    if bytes.len() < prefix_sharp_cnt + expect_postfix_sharp_cnt + 3 {
        working_set.error(ParseError::Unclosed('\''.into(), span));
        return garbage(span);
    }

    // check for unbalanced # and single quotes.
    let postfix_bytes = &bytes[bytes.len() - expect_postfix_sharp_cnt..bytes.len()];
    if postfix_bytes.iter().any(|b| *b != b'#') {
        working_set.error(ParseError::Unbalanced(
            "prefix #".to_string(),
            "postfix #".to_string(),
            span,
        ));
        return garbage(span);
    }
    // check for unblanaced single quotes.
    if bytes[1 + prefix_sharp_cnt] != b'\''
        || bytes[bytes.len() - expect_postfix_sharp_cnt - 1] != b'\''
    {
        working_set.error(ParseError::Unclosed('\''.into(), span));
        return garbage(span);
    }

    let bytes = &bytes[prefix_sharp_cnt + 1 + 1..bytes.len() - 1 - prefix_sharp_cnt];
    if let Ok(token) = String::from_utf8(bytes.into()) {
        Expression {
            expr: Expr::RawString(token),
            span,
            ty: Type::String,
            custom_completion: None,
        }
    } else {
        working_set.error(ParseError::Expected("utf8 raw-string", span));
        garbage(span)
    }
}

pub fn parse_paren_expr(
    working_set: &mut StateWorkingSet,
    span: Span,
    shape: &SyntaxShape,
) -> Expression {
    let starting_error_count = working_set.parse_errors.len();

    let expr = parse_range(working_set, span);

    if starting_error_count == working_set.parse_errors.len() {
        expr
    } else {
        working_set.parse_errors.truncate(starting_error_count);

        if matches!(shape, SyntaxShape::Signature) {
            parse_signature(working_set, span)
        } else {
            parse_full_cell_path(working_set, None, span)
        }
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
        return Expression::garbage(span);
    }

    let bytes = working_set.get_span_contents(Span::new(span.start + 1, span.end - 1));
    let (tokens, _) = lex(bytes, span.start + 1, &[b'\r', b'\n', b'\t'], &[b':'], true);

    let second_token = tokens
        .first()
        .map(|token| working_set.get_span_contents(token.span));

    let second_token_contents = tokens.first().map(|token| token.contents);

    let third_token = tokens
        .get(1)
        .map(|token| working_set.get_span_contents(token.span));

    if second_token.is_none() {
        // If we're empty, that means an empty record or closure
        if matches!(shape, SyntaxShape::Closure(_)) {
            parse_closure_expression(working_set, shape, span)
        } else if matches!(shape, SyntaxShape::Block) {
            parse_block_expression(working_set, span)
        } else if matches!(shape, SyntaxShape::MatchBlock) {
            parse_match_block_expression(working_set, span)
        } else {
            parse_record(working_set, span)
        }
    } else if matches!(second_token_contents, Some(TokenContents::Pipe))
        || matches!(second_token_contents, Some(TokenContents::PipePipe))
    {
        parse_closure_expression(working_set, shape, span)
    } else if matches!(third_token, Some(b":")) {
        parse_full_cell_path(working_set, None, span)
    } else if matches!(shape, SyntaxShape::Closure(_)) {
        parse_closure_expression(working_set, shape, span)
    } else if matches!(shape, SyntaxShape::Block) {
        parse_block_expression(working_set, span)
    } else if matches!(shape, SyntaxShape::MatchBlock) {
        parse_match_block_expression(working_set, span)
    } else if second_token.is_some_and(|c| {
        c.len() > 3 && c.starts_with(b"...") && (c[3] == b'$' || c[3] == b'{' || c[3] == b'(')
    }) {
        parse_record(working_set, span)
    } else if matches!(shape, SyntaxShape::Any) {
        parse_closure_expression(working_set, shape, span)
    } else {
        working_set.error(ParseError::ExpectedWithStringMsg(
            format!("non-block value: {shape}"),
            span,
        ));

        Expression::garbage(span)
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

    let inner_span = Span::new(start, end);
    let contents = working_set.get_span_contents(inner_span).to_vec();

    let mut output = vec![];
    let mut mode = InterpolationMode::String;
    let mut token_start = start;
    let mut delimiter_stack = vec![];

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

            if current_byte == b'(' && (!double_quote || preceding_consecutive_backslashes % 2 == 0)
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

                    output.push(Expression {
                        expr: Expr::String(String::from_utf8_lossy(&str_contents).to_string()),
                        span,
                        ty: Type::String,
                        custom_completion: None,
                    });
                    token_start = b;
                }
            }
        }

        if mode == InterpolationMode::Expression {
            let byte = current_byte;
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
                        let span = Span::new(token_start, b + 1);

                        let expr = parse_full_cell_path(working_set, None, span);
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
                let span = Span::new(token_start, end);

                let expr = parse_full_cell_path(working_set, None, span);
                output.push(expr);
            }
        }
    }

    Expression {
        expr: Expr::StringInterpolation(output),
        span,
        ty: Type::String,
        custom_completion: None,
    }
}

pub fn parse_variable_expr(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let contents = working_set.get_span_contents(span);

    if contents == b"$nu" {
        return Expression {
            expr: Expr::Var(nu_protocol::NU_VARIABLE_ID),
            span,
            ty: Type::Any,
            custom_completion: None,
        };
    } else if contents == b"$in" {
        return Expression {
            expr: Expr::Var(nu_protocol::IN_VARIABLE_ID),
            span,
            ty: Type::Any,
            custom_completion: None,
        };
    } else if contents == b"$env" {
        return Expression {
            expr: Expr::Var(nu_protocol::ENV_VARIABLE_ID),
            span,
            ty: Type::Any,
            custom_completion: None,
        };
    }

    let name = if contents.starts_with(b"$") {
        String::from_utf8_lossy(&contents[1..]).to_string()
    } else {
        String::from_utf8_lossy(contents).to_string()
    };

    if let Some(id) = parse_variable(working_set, span) {
        Expression {
            expr: Expr::Var(id),
            span,
            ty: working_set.get_variable(id).ty.clone(),
            custom_completion: None,
        }
    } else if working_set.get_env_var(&name).is_some() {
        working_set.error(ParseError::EnvVarNotVar(name, span));
        garbage(span)
    } else {
        let ws = &*working_set;
        let suggestion = DidYouMean::new(&ws.list_variables(), ws.get_span_contents(span));
        working_set.error(ParseError::VariableNotFound(suggestion, span));
        garbage(span)
    }
}

pub fn parse_cell_path(
    working_set: &mut StateWorkingSet,
    tokens: impl Iterator<Item = Token>,
    expect_dot: bool,
) -> Vec<PathMember> {
    enum TokenType {
        Dot,           // .
        QuestionOrDot, // ? or .
        PathMember,    // an int or string, like `1` or `foo`
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

        match expected_token {
            TokenType::Dot => {
                if bytes.len() != 1 || bytes[0] != b'.' {
                    working_set.error(ParseError::Expected(".", path_element.span));
                    return tail;
                }
                expected_token = TokenType::PathMember;
            }
            TokenType::QuestionOrDot => {
                if bytes.len() == 1 && bytes[0] == b'.' {
                    expected_token = TokenType::PathMember;
                } else if bytes.len() == 1 && bytes[0] == b'?' {
                    if let Some(last) = tail.last_mut() {
                        match last {
                            PathMember::String {
                                ref mut optional, ..
                            } => *optional = true,
                            PathMember::Int {
                                ref mut optional, ..
                            } => *optional = true,
                        }
                    }
                    expected_token = TokenType::Dot;
                } else {
                    working_set.error(ParseError::Expected(". or ?", path_element.span));
                    return tail;
                }
            }
            TokenType::PathMember => {
                let starting_error_count = working_set.parse_errors.len();

                let expr = parse_int(working_set, path_element.span);
                working_set.parse_errors.truncate(starting_error_count);

                match expr {
                    Expression {
                        expr: Expr::Int(val),
                        span,
                        ..
                    } => tail.push(PathMember::Int {
                        val: val as usize,
                        span,
                        optional: false,
                    }),
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
                                });
                            }
                            _ => {
                                working_set
                                    .error(ParseError::Expected("string", path_element.span));
                                return tail;
                            }
                        }
                    }
                }
                expected_token = TokenType::QuestionOrDot;
            }
        }
    }

    tail
}

pub fn parse_simple_cell_path(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let source = working_set.get_span_contents(span);

    let (tokens, err) = lex(source, span.start, &[b'\n', b'\r'], &[b'.', b'?'], true);
    if let Some(err) = err {
        working_set.error(err)
    }

    let tokens = tokens.into_iter().peekable();

    let cell_path = parse_cell_path(working_set, tokens, false);

    Expression {
        expr: Expr::CellPath(CellPath { members: cell_path }),
        span,
        ty: Type::CellPath,
        custom_completion: None,
    }
}

pub fn parse_full_cell_path(
    working_set: &mut StateWorkingSet,
    implicit_head: Option<VarId>,
    span: Span,
) -> Expression {
    trace!("parsing: full cell path");
    let full_cell_span = span;
    let source = working_set.get_span_contents(span);

    let (tokens, err) = lex(source, span.start, &[b'\n', b'\r'], &[b'.', b'?'], true);
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

            if bytes.starts_with(b"(") {
                start += 1;
            }
            if bytes.ends_with(b")") {
                end -= 1;
            } else {
                working_set.error(ParseError::Unclosed(")".into(), Span::new(end, end)));
            }

            let span = Span::new(start, end);

            let source = working_set.get_span_contents(span);

            let (output, err) = lex(source, span.start, &[b'\n', b'\r'], &[], true);
            if let Some(err) = err {
                working_set.error(err)
            }

            // Creating a Type scope to parse the new block. This will keep track of
            // the previous input type found in that block
            let output = parse_block(working_set, &output, span, true, true);

            let ty = output.output_type();

            let block_id = working_set.add_block(Arc::new(output));
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

            let output = parse_table_expression(working_set, head.span);

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
                Expression {
                    expr: Expr::Var(var_id),
                    span: head.span,
                    ty: Type::Any,
                    custom_completion: None,
                },
                false,
            )
        } else {
            working_set.error(ParseError::Mismatch(
                "variable or subexpression".into(),
                String::from_utf8_lossy(bytes).to_string(),
                span,
            ));
            return garbage(span);
        };

        let tail = parse_cell_path(working_set, tokens, expect_dot);

        Expression {
            // FIXME: Get the type of the data at the tail using follow_cell_path() (or something)
            ty: if !tail.is_empty() {
                // Until the aforementioned fix is implemented, this is necessary to allow mutable list upserts
                // such as $a.1 = 2 to work correctly.
                Type::Any
            } else {
                head.ty.clone()
            },
            expr: Expr::FullCellPath(Box::new(FullCellPath { head, tail })),
            span: full_cell_span,
            custom_completion: None,
        }
    } else {
        garbage(span)
    }
}

pub fn parse_directory(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let bytes = working_set.get_span_contents(span);
    let quoted = is_quoted(bytes);
    let (token, err) = unescape_unquote_string(bytes, span);
    trace!("parsing: directory");

    if err.is_none() {
        trace!("-- found {}", token);

        Expression {
            expr: Expr::Directory(token, quoted),
            span,
            ty: Type::String,
            custom_completion: None,
        }
    } else {
        working_set.error(ParseError::Expected("directory", span));

        garbage(span)
    }
}

pub fn parse_filepath(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let bytes = working_set.get_span_contents(span);
    let quoted = is_quoted(bytes);
    let (token, err) = unescape_unquote_string(bytes, span);
    trace!("parsing: filepath");

    if err.is_none() {
        trace!("-- found {}", token);

        Expression {
            expr: Expr::Filepath(token, quoted),
            span,
            ty: Type::String,
            custom_completion: None,
        }
    } else {
        working_set.error(ParseError::Expected("filepath", span));

        garbage(span)
    }
}
/// Parse a datetime type, eg '2022-02-02'
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
        return garbage(span);
    }

    let token = String::from_utf8_lossy(bytes).to_string();

    if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(&token) {
        return Expression {
            expr: Expr::DateTime(datetime),
            span,
            ty: Type::Date,
            custom_completion: None,
        };
    }

    // Just the date
    let just_date = token.clone() + "T00:00:00+00:00";
    if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(&just_date) {
        return Expression {
            expr: Expr::DateTime(datetime),
            span,
            ty: Type::Date,
            custom_completion: None,
        };
    }

    // Date and time, assume UTC
    let datetime = token + "+00:00";
    if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(&datetime) {
        return Expression {
            expr: Expr::DateTime(datetime),
            span,
            ty: Type::Date,
            custom_completion: None,
        };
    }

    working_set.error(ParseError::Expected("datetime", span));

    garbage(span)
}

/// Parse a duration type, eg '10day'
pub fn parse_duration(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: duration");

    let bytes = working_set.get_span_contents(span);

    match parse_unit_value(bytes, span, DURATION_UNIT_GROUPS, Type::Duration, |x| x) {
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

    match parse_unit_value(bytes, span, FILESIZE_UNIT_GROUPS, Type::Filesize, |x| {
        x.to_ascii_uppercase()
    }) {
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
    transform: fn(String) -> String,
) -> Option<ParseUnitResult<'res>> {
    if bytes.len() < 2
        || !(bytes[0].is_ascii_digit() || (bytes[0] == b'-' && bytes[1].is_ascii_digit()))
    {
        return None;
    }

    let value = transform(String::from_utf8_lossy(bytes).into());

    if let Some((unit, name, convert)) = unit_groups.iter().find(|x| value.ends_with(x.1)) {
        let lhs_len = value.len() - name.len();
        let lhs = strip_underscores(value[..lhs_len].as_bytes());
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

        let (num, unit) = match convert {
            Some(convert_to) => (
                ((number_part * convert_to.1 as f64) + (decimal_part * convert_to.1 as f64)) as i64,
                convert_to.0,
            ),
            None => (number_part as i64, *unit),
        };

        trace!("-- found {} {:?}", num, unit);
        let value = ValueWithUnit {
            expr: Expression {
                expr: Expr::Int(num),
                span: lhs_span,
                ty: Type::Number,
                custom_completion: None,
            },
            unit: Spanned {
                item: unit,
                span: unit_span,
            },
        };
        let expr = Expression {
            expr: Expr::ValueWithUnit(Box::new(value)),
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

pub fn parse_glob_pattern(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let bytes = working_set.get_span_contents(span);
    let quoted = is_quoted(bytes);
    let (token, err) = unescape_unquote_string(bytes, span);
    trace!("parsing: glob pattern");

    if err.is_none() {
        trace!("-- found {}", token);

        Expression {
            expr: Expr::GlobPattern(token, quoted),
            span,
            ty: Type::Glob,
            custom_completion: None,
        }
    } else {
        working_set.error(ParseError::Expected("glob pattern string", span));

        garbage(span)
    }
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
                Some(b'u') => {
                    let mut digits = String::with_capacity(10);
                    let mut cur_idx = idx + 1; // index of first beyond current end of token

                    if let Some(b'{') = bytes.get(idx + 1) {
                        cur_idx = idx + 2;
                        loop {
                            match bytes.get(cur_idx) {
                                Some(b'}') => {
                                    cur_idx += 1;
                                    break;
                                }
                                Some(c) => {
                                    digits.push(*c as char);
                                    cur_idx += 1;
                                }
                                _ => {
                                    error = error.or(Some(ParseError::InvalidLiteral(
                                        "missing '}' for unicode escape '\\u{X...}'".into(),
                                        "string".into(),
                                        Span::new(span.start + idx, span.end),
                                    )));
                                    break 'us_loop;
                                }
                            }
                        }
                    }

                    if (1..=6).contains(&digits.len()) {
                        let int = u32::from_str_radix(&digits, 16);

                        if let Ok(int) = int {
                            if int <= 0x10ffff {
                                let result = char::from_u32(int);

                                if let Some(result) = result {
                                    let mut buffer = vec![0; 4];
                                    let result = result.encode_utf8(&mut buffer);

                                    for elem in result.bytes() {
                                        output.push(elem);
                                    }

                                    idx = cur_idx;
                                    continue 'us_loop;
                                }
                            }
                        }
                    }
                    // fall through -- escape not accepted above, must be error.
                    error = error.or(Some(ParseError::InvalidLiteral(
                            "invalid unicode escape '\\u{X...}', must be 1-6 hex digits, max value 10FFFF".into(),
                            "string".into(),
                            Span::new(span.start + idx, span.end),
                    )));
                    break 'us_loop;
                }

                _ => {
                    error = error.or(Some(ParseError::InvalidLiteral(
                        "unrecognized escape after '\\'".into(),
                        "string".into(),
                        Span::new(span.start + idx, span.end),
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

pub fn parse_string(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: string");

    let bytes = working_set.get_span_contents(span);

    if bytes.is_empty() {
        working_set.error(ParseError::Expected("String", span));
        return Expression::garbage(span);
    }

    // Check for bare word interpolation
    if bytes[0] != b'\'' && bytes[0] != b'"' && bytes[0] != b'`' && bytes.contains(&b'(') {
        return parse_string_interpolation(working_set, span);
    }

    let (s, err) = unescape_unquote_string(bytes, span);
    if let Some(err) = err {
        working_set.error(err);
    }

    Expression {
        expr: Expr::String(s),
        span,
        ty: Type::String,
        custom_completion: None,
    }
}

fn is_quoted(bytes: &[u8]) -> bool {
    (bytes.starts_with(b"\"") && bytes.ends_with(b"\"") && bytes.len() > 1)
        || (bytes.starts_with(b"\'") && bytes.ends_with(b"\'") && bytes.len() > 1)
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
            working_set.error(ParseError::Unclosed("\"".into(), span));
            return garbage(span);
        }
        if bytes.starts_with(b"\'") && (bytes.len() == 1 || !bytes.ends_with(b"\'")) {
            working_set.error(ParseError::Unclosed("\'".into(), span));
            return garbage(span);
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
            Expression {
                expr: Expr::String(token),
                span,
                ty: Type::String,
                custom_completion: None,
            }
        } else if token.contains(' ') {
            working_set.error(ParseError::Expected("string", span));

            garbage(span)
        } else {
            Expression {
                expr: Expr::String(token),
                span,
                ty: Type::String,
                custom_completion: None,
            }
        }
    } else {
        working_set.error(ParseError::Expected("string", span));
        garbage(span)
    }
}

pub fn parse_import_pattern(working_set: &mut StateWorkingSet, spans: &[Span]) -> Expression {
    let Some(head_span) = spans.first() else {
        working_set.error(ParseError::WrongImportPattern(
            "needs at least one component of import pattern".to_string(),
            Span::concat(spans),
        ));
        return garbage(Span::concat(spans));
    };

    let head_expr = parse_value(working_set, *head_span, &SyntaxShape::Any);

    let (maybe_module_id, head_name) = match eval_constant(working_set, &head_expr) {
        Ok(val) => match val.coerce_into_string() {
            Ok(s) => (working_set.find_module(s.as_bytes()), s.into_bytes()),
            Err(err) => {
                working_set.error(err.wrap(working_set, Span::concat(spans)));
                return garbage(Span::concat(spans));
            }
        },
        Err(err) => {
            working_set.error(err.wrap(working_set, Span::concat(spans)));
            return garbage(Span::concat(spans));
        }
    };

    let mut import_pattern = ImportPattern {
        head: ImportPatternHead {
            name: head_name,
            id: maybe_module_id,
            span: *head_span,
        },
        members: vec![],
        hidden: HashSet::new(),
        constants: vec![],
    };

    if spans.len() > 1 {
        let mut leaf_member_span = None;

        for tail_span in spans[1..].iter() {
            if let Some(prev_span) = leaf_member_span {
                let what = if working_set.get_span_contents(prev_span) == b"*" {
                    "glob"
                } else {
                    "list"
                };
                working_set.error(ParseError::WrongImportPattern(
                    format!(
                        "{} member can be only at the end of an import pattern",
                        what
                    ),
                    prev_span,
                ));
                return Expression {
                    expr: Expr::ImportPattern(Box::new(import_pattern)),
                    span: prev_span,
                    ty: Type::List(Box::new(Type::String)),
                    custom_completion: None,
                };
            }

            let tail = working_set.get_span_contents(*tail_span);

            if tail == b"*" {
                import_pattern
                    .members
                    .push(ImportPatternMember::Glob { span: *tail_span });

                leaf_member_span = Some(*tail_span);
            } else if tail.starts_with(b"[") {
                let result = parse_list_expression(working_set, *tail_span, &SyntaxShape::String);

                let mut output = vec![];

                if let Expression {
                    expr: Expr::List(list),
                    ..
                } = result
                {
                    for item in list {
                        match item {
                            ListItem::Item(expr) => {
                                let contents = working_set.get_span_contents(expr.span);
                                output.push((trim_quotes(contents).to_vec(), expr.span));
                            }
                            ListItem::Spread(_, spread) => {
                                working_set.error(ParseError::WrongImportPattern(
                                    "cannot spread in an import pattern".into(),
                                    spread.span,
                                ))
                            }
                        }
                    }

                    import_pattern
                        .members
                        .push(ImportPatternMember::List { names: output });
                } else {
                    working_set.error(ParseError::ExportNotFound(result.span));
                    return Expression {
                        expr: Expr::ImportPattern(Box::new(import_pattern)),
                        span: Span::concat(spans),
                        ty: Type::List(Box::new(Type::String)),
                        custom_completion: None,
                    };
                }

                leaf_member_span = Some(*tail_span);
            } else {
                let tail = trim_quotes(tail);

                import_pattern.members.push(ImportPatternMember::Name {
                    name: tail.to_vec(),
                    span: *tail_span,
                });
            }
        }
    }

    Expression {
        expr: Expr::ImportPattern(Box::new(import_pattern)),
        span: Span::concat(&spans[1..]),
        ty: Type::List(Box::new(Type::String)),
        custom_completion: None,
    }
}

/// Parse `spans[spans_idx..]` into a variable, with optional type annotation.
/// If the name of the variable ends with a colon (no space in-between allowed), then a type annotation
/// can appear after the variable, in which case the colon is stripped from the name of the variable.
/// `spans_idx` is updated to point to the last span that has been parsed.
pub fn parse_var_with_opt_type(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    spans_idx: &mut usize,
    mutable: bool,
) -> (Expression, Option<Type>) {
    let bytes = working_set.get_span_contents(spans[*spans_idx]).to_vec();

    if bytes.contains(&b' ')
        || bytes.contains(&b'"')
        || bytes.contains(&b'\'')
        || bytes.contains(&b'`')
    {
        working_set.error(ParseError::VariableNotValid(spans[*spans_idx]));
        return (garbage(spans[*spans_idx]), None);
    }

    if bytes.ends_with(b":") {
        // We end with colon, so the next span should be the type
        if *spans_idx + 1 < spans.len() {
            let span_beginning = *spans_idx;
            *spans_idx += 1;
            // signature like record<a: int b: int> is broken into multiple spans due to
            // whitespaces. Collect the rest into one span and work on it
            let full_span = Span::concat(&spans[*spans_idx..]);
            let type_bytes = working_set.get_span_contents(full_span).to_vec();

            let (tokens, parse_error) =
                lex_signature(&type_bytes, full_span.start, &[b','], &[], true);

            if let Some(parse_error) = parse_error {
                working_set.error(parse_error);
            }

            let ty = parse_type(working_set, &type_bytes, tokens[0].span);
            *spans_idx = spans.len() - 1;

            let var_name = bytes[0..(bytes.len() - 1)].to_vec();

            if !is_variable(&var_name) {
                working_set.error(ParseError::Expected(
                    "valid variable name",
                    spans[*spans_idx - 1],
                ));
                return (garbage(spans[*spans_idx - 1]), None);
            }

            let id = working_set.add_variable(var_name, spans[*spans_idx - 1], ty.clone(), mutable);

            (
                Expression {
                    expr: Expr::VarDecl(id),
                    span: Span::concat(&spans[span_beginning..*spans_idx + 1]),
                    ty: ty.clone(),
                    custom_completion: None,
                },
                Some(ty),
            )
        } else {
            let var_name = bytes[0..(bytes.len() - 1)].to_vec();

            if !is_variable(&var_name) {
                working_set.error(ParseError::Expected(
                    "valid variable name",
                    spans[*spans_idx],
                ));
                return (garbage(spans[*spans_idx]), None);
            }

            let id = working_set.add_variable(var_name, spans[*spans_idx], Type::Any, mutable);

            working_set.error(ParseError::MissingType(spans[*spans_idx]));
            (
                Expression {
                    expr: Expr::VarDecl(id),
                    span: spans[*spans_idx],
                    ty: Type::Any,
                    custom_completion: None,
                },
                None,
            )
        }
    } else {
        let var_name = bytes;

        if !is_variable(&var_name) {
            working_set.error(ParseError::Expected(
                "valid variable name",
                spans[*spans_idx],
            ));
            return (garbage(spans[*spans_idx]), None);
        }

        let id = working_set.add_variable(
            var_name,
            Span::concat(&spans[*spans_idx..*spans_idx + 1]),
            Type::Any,
            mutable,
        );

        (
            Expression {
                expr: Expr::VarDecl(id),
                span: spans[*spans_idx],
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
) {
    trace!("parsing: expanding to cell path");
    if let Expression {
        expr: Expr::String(_),
        span,
        ..
    } = expression
    {
        // Re-parse the string as if it were a cell-path
        let new_expression = parse_full_cell_path(working_set, Some(var_id), *span);

        *expression = new_expression;
    }

    if let Expression {
        expr: Expr::UnaryNot(inner),
        ..
    } = expression
    {
        expand_to_cell_path(working_set, inner, var_id);
    }
}

pub fn parse_input_output_types(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> Vec<(Type, Type)> {
    let mut full_span = Span::concat(spans);

    let mut bytes = working_set.get_span_contents(full_span);

    if bytes.starts_with(b"[") {
        bytes = &bytes[1..];
        full_span.start += 1;
    }

    if bytes.ends_with(b"]") {
        bytes = &bytes[..(bytes.len() - 1)];
        full_span.end -= 1;
    }

    let (tokens, parse_error) =
        lex_signature(bytes, full_span.start, &[b'\n', b'\r', b','], &[], true);

    if let Some(parse_error) = parse_error {
        working_set.error(parse_error);
    }

    let mut output = vec![];

    let mut idx = 0;
    while idx < tokens.len() {
        let type_bytes = working_set.get_span_contents(tokens[idx].span).to_vec();
        let input_type = parse_type(working_set, &type_bytes, tokens[idx].span);

        idx += 1;
        if idx >= tokens.len() {
            working_set.error(ParseError::Expected(
                "arrow (->)",
                Span::new(tokens[idx - 1].span.end, tokens[idx - 1].span.end),
            ));
            break;
        }

        let arrow = working_set.get_span_contents(tokens[idx].span);
        if arrow != b"->" {
            working_set.error(ParseError::Expected("arrow (->)", tokens[idx].span));
        }

        idx += 1;
        if idx >= tokens.len() {
            working_set.error(ParseError::MissingType(Span::new(
                tokens[idx - 1].span.end,
                tokens[idx - 1].span.end,
            )));
            break;
        }

        let type_bytes = working_set.get_span_contents(tokens[idx].span).to_vec();
        let output_type = parse_type(working_set, &type_bytes, tokens[idx].span);

        output.push((input_type, output_type));

        idx += 1;
    }

    output
}

pub fn parse_full_signature(working_set: &mut StateWorkingSet, spans: &[Span]) -> Expression {
    let arg_signature = working_set.get_span_contents(spans[0]);

    if arg_signature.ends_with(b":") {
        let mut arg_signature =
            parse_signature(working_set, Span::new(spans[0].start, spans[0].end - 1));

        let input_output_types = parse_input_output_types(working_set, &spans[1..]);

        if let Expression {
            expr: Expr::Signature(sig),
            span: expr_span,
            ..
        } = &mut arg_signature
        {
            sig.input_output_types = input_output_types;
            expr_span.end = Span::concat(&spans[1..]).end;
        }
        arg_signature
    } else {
        parse_signature(working_set, spans[0])
    }
}

pub fn parse_row_condition(working_set: &mut StateWorkingSet, spans: &[Span]) -> Expression {
    let var_id = working_set.add_variable(b"$it".to_vec(), Span::concat(spans), Type::Any, false);
    let expression = parse_math_expression(working_set, spans, Some(var_id));
    let span = Span::concat(spans);

    let block_id = match expression.expr {
        Expr::Block(block_id) => block_id,
        Expr::Closure(block_id) => block_id,
        _ => {
            // We have an expression, so let's convert this into a block.
            let mut block = Block::new();
            let mut pipeline = Pipeline::new();
            pipeline.elements.push(PipelineElement {
                pipe: None,
                expr: expression,
                redirection: None,
            });

            block.pipelines.push(pipeline);

            block.signature.required_positional.push(PositionalArg {
                name: "$it".into(),
                desc: "row condition".into(),
                shape: SyntaxShape::Any,
                var_id: Some(var_id),
                default_value: None,
            });

            working_set.add_block(Arc::new(block))
        }
    };

    Expression {
        ty: Type::Bool,
        span,
        expr: Expr::RowCondition(block_id),
        custom_completion: None,
    }
}

pub fn parse_signature(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let bytes = working_set.get_span_contents(span);

    let mut start = span.start;
    let mut end = span.end;

    let mut has_paren = false;

    if bytes.starts_with(b"[") {
        start += 1;
    } else if bytes.starts_with(b"(") {
        has_paren = true;
        start += 1;
    } else {
        working_set.error(ParseError::Expected("[ or (", Span::new(start, start + 1)));
        return garbage(span);
    }

    if (has_paren && bytes.ends_with(b")")) || (!has_paren && bytes.ends_with(b"]")) {
        end -= 1;
    } else {
        working_set.error(ParseError::Unclosed("] or )".into(), Span::new(end, end)));
    }

    let sig = parse_signature_helper(working_set, Span::new(start, end));

    Expression {
        expr: Expr::Signature(sig),
        span,
        ty: Type::Signature,
        custom_completion: None,
    }
}

pub fn parse_signature_helper(working_set: &mut StateWorkingSet, span: Span) -> Box<Signature> {
    enum ParseMode {
        Arg,
        AfterCommaArg,
        Type,
        DefaultValue,
    }

    #[derive(Debug)]
    enum Arg {
        Positional {
            arg: PositionalArg,
            required: bool,
            type_annotated: bool,
        },
        RestPositional(PositionalArg),
        Flag {
            flag: Flag,
            type_annotated: bool,
        },
    }

    let source = working_set.get_span_contents(span);

    let (output, err) = lex_signature(
        source,
        span.start,
        &[b'\n', b'\r'],
        &[b':', b'=', b','],
        false,
    );
    if let Some(err) = err {
        working_set.error(err);
    }

    let mut args: Vec<Arg> = vec![];
    let mut parse_mode = ParseMode::Arg;

    for token in &output {
        match token {
            Token {
                contents: crate::TokenContents::Item,
                span,
            } => {
                let span = *span;
                let contents = working_set.get_span_contents(span).to_vec();

                // The : symbol separates types
                if contents == b":" {
                    match parse_mode {
                        ParseMode::Arg => {
                            parse_mode = ParseMode::Type;
                        }
                        ParseMode::AfterCommaArg => {
                            working_set.error(ParseError::Expected("parameter or flag", span));
                        }
                        ParseMode::Type | ParseMode::DefaultValue => {
                            // We're seeing two types for the same thing for some reason, error
                            working_set.error(ParseError::Expected("type", span));
                        }
                    }
                }
                // The = symbol separates a variable from its default value
                else if contents == b"=" {
                    match parse_mode {
                        ParseMode::Type | ParseMode::Arg => {
                            parse_mode = ParseMode::DefaultValue;
                        }
                        ParseMode::AfterCommaArg => {
                            working_set.error(ParseError::Expected("parameter or flag", span));
                        }
                        ParseMode::DefaultValue => {
                            // We're seeing two default values for some reason, error
                            working_set.error(ParseError::Expected("default value", span));
                        }
                    }
                }
                // The , symbol separates params only
                else if contents == b"," {
                    match parse_mode {
                        ParseMode::Arg => parse_mode = ParseMode::AfterCommaArg,
                        ParseMode::AfterCommaArg => {
                            working_set.error(ParseError::Expected("parameter or flag", span));
                        }
                        ParseMode::Type => {
                            working_set.error(ParseError::Expected("type", span));
                        }
                        ParseMode::DefaultValue => {
                            working_set.error(ParseError::Expected("default value", span));
                        }
                    }
                } else {
                    match parse_mode {
                        ParseMode::Arg | ParseMode::AfterCommaArg => {
                            // Long flag with optional short form following with no whitespace, e.g. --output, --age(-a)
                            if contents.starts_with(b"--") && contents.len() > 2 {
                                // Split the long flag from the short flag with the ( character as delimiter.
                                // The trailing ) is removed further down.
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
                                    working_set.error(ParseError::Expected(
                                        "valid variable name for this long flag",
                                        span,
                                    ))
                                }

                                let var_id =
                                    working_set.add_variable(variable_name, span, Type::Any, false);

                                // If there's no short flag, exit now. Otherwise, parse it.
                                if flags.len() == 1 {
                                    args.push(Arg::Flag {
                                        flag: Flag {
                                            arg: None,
                                            desc: String::new(),
                                            long,
                                            short: None,
                                            required: false,
                                            var_id: Some(var_id),
                                            default_value: None,
                                        },
                                        type_annotated: false,
                                    });
                                } else if flags.len() >= 3 {
                                    working_set.error(ParseError::Expected(
                                        "only one short flag alternative",
                                        span,
                                    ));
                                } else {
                                    let short_flag = &flags[1];
                                    let short_flag = if !short_flag.starts_with(b"-")
                                        || !short_flag.ends_with(b")")
                                    {
                                        working_set.error(ParseError::Expected(
                                            "short flag alternative for the long flag",
                                            span,
                                        ));
                                        short_flag
                                    } else {
                                        // Obtain the flag's name by removing the starting - and trailing )
                                        &short_flag[1..(short_flag.len() - 1)]
                                    };
                                    // Note that it is currently possible to make a short flag with non-alphanumeric characters,
                                    // like -).

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
                                        working_set.error(ParseError::Expected(
                                            "valid variable name for this short flag",
                                            span,
                                        ))
                                    }

                                    let var_id = working_set.add_variable(
                                        variable_name,
                                        span,
                                        Type::Any,
                                        false,
                                    );

                                    if chars.len() == 1 {
                                        args.push(Arg::Flag {
                                            flag: Flag {
                                                arg: None,
                                                desc: String::new(),
                                                long,
                                                short: Some(chars[0]),
                                                required: false,
                                                var_id: Some(var_id),
                                                default_value: None,
                                            },
                                            type_annotated: false,
                                        });
                                    } else {
                                        working_set.error(ParseError::Expected("short flag", span));
                                    }
                                }
                                parse_mode = ParseMode::Arg;
                            }
                            // Mandatory short flag, e.g. -e (must be one character)
                            else if contents.starts_with(b"-") && contents.len() > 1 {
                                let short_flag = &contents[1..];
                                let short_flag = String::from_utf8_lossy(short_flag).to_string();
                                let chars: Vec<char> = short_flag.chars().collect();

                                if chars.len() > 1 {
                                    working_set.error(ParseError::Expected("short flag", span));
                                }

                                let mut encoded_var_name = vec![0u8; 4];
                                let len = chars[0].encode_utf8(&mut encoded_var_name).len();
                                let variable_name = encoded_var_name[0..len].to_vec();

                                if !is_variable(&variable_name) {
                                    working_set.error(ParseError::Expected(
                                        "valid variable name for this short flag",
                                        span,
                                    ))
                                }

                                let var_id =
                                    working_set.add_variable(variable_name, span, Type::Any, false);

                                args.push(Arg::Flag {
                                    flag: Flag {
                                        arg: None,
                                        desc: String::new(),
                                        long: String::new(),
                                        short: Some(chars[0]),
                                        required: false,
                                        var_id: Some(var_id),
                                        default_value: None,
                                    },
                                    type_annotated: false,
                                });
                                parse_mode = ParseMode::Arg;
                            }
                            // Short flag alias for long flag, e.g. --b (-a)
                            // This is the same as the short flag in --b(-a)
                            else if contents.starts_with(b"(-") {
                                if matches!(parse_mode, ParseMode::AfterCommaArg) {
                                    working_set
                                        .error(ParseError::Expected("parameter or flag", span));
                                }
                                let short_flag = &contents[2..];

                                let short_flag = if !short_flag.ends_with(b")") {
                                    working_set.error(ParseError::Expected("short flag", span));
                                    short_flag
                                } else {
                                    &short_flag[..(short_flag.len() - 1)]
                                };

                                let short_flag = String::from_utf8_lossy(short_flag).to_string();
                                let chars: Vec<char> = short_flag.chars().collect();

                                if chars.len() == 1 {
                                    match args.last_mut() {
                                        Some(Arg::Flag { flag, .. }) => {
                                            if flag.short.is_some() {
                                                working_set.error(ParseError::Expected(
                                                    "one short flag",
                                                    span,
                                                ));
                                            } else {
                                                flag.short = Some(chars[0]);
                                            }
                                        }
                                        _ => {
                                            working_set
                                                .error(ParseError::Expected("unknown flag", span));
                                        }
                                    }
                                } else {
                                    working_set.error(ParseError::Expected("short flag", span));
                                }
                            }
                            // Positional arg, optional
                            else if contents.ends_with(b"?") {
                                let contents: Vec<_> = contents[..(contents.len() - 1)].into();
                                let name = String::from_utf8_lossy(&contents).to_string();

                                if !is_variable(&contents) {
                                    working_set.error(ParseError::Expected(
                                        "valid variable name for this optional parameter",
                                        span,
                                    ))
                                }

                                let var_id =
                                    working_set.add_variable(contents, span, Type::Any, false);

                                args.push(Arg::Positional {
                                    arg: PositionalArg {
                                        desc: String::new(),
                                        name,
                                        shape: SyntaxShape::Any,
                                        var_id: Some(var_id),
                                        default_value: None,
                                    },
                                    required: false,
                                    type_annotated: false,
                                });
                                parse_mode = ParseMode::Arg;
                            }
                            // Rest param
                            else if let Some(contents) = contents.strip_prefix(b"...") {
                                let name = String::from_utf8_lossy(contents).to_string();
                                let contents_vec: Vec<u8> = contents.to_vec();

                                if !is_variable(&contents_vec) {
                                    working_set.error(ParseError::Expected(
                                        "valid variable name for this rest parameter",
                                        span,
                                    ))
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
                                parse_mode = ParseMode::Arg;
                            }
                            // Normal param
                            else {
                                let name = String::from_utf8_lossy(&contents).to_string();
                                let contents_vec = contents.to_vec();

                                if !is_variable(&contents_vec) {
                                    working_set.error(ParseError::Expected(
                                        "valid variable name for this parameter",
                                        span,
                                    ))
                                }

                                let var_id =
                                    working_set.add_variable(contents_vec, span, Type::Any, false);

                                // Positional arg, required
                                args.push(Arg::Positional {
                                    arg: PositionalArg {
                                        desc: String::new(),
                                        name,
                                        shape: SyntaxShape::Any,
                                        var_id: Some(var_id),
                                        default_value: None,
                                    },
                                    required: true,
                                    type_annotated: false,
                                });
                                parse_mode = ParseMode::Arg;
                            }
                        }
                        ParseMode::Type => {
                            if let Some(last) = args.last_mut() {
                                let syntax_shape = parse_shape_name(
                                    working_set,
                                    &contents,
                                    span,
                                    ShapeDescriptorUse::Argument,
                                );
                                //TODO check if we're replacing a custom parameter already
                                match last {
                                    Arg::Positional {
                                        arg: PositionalArg { shape, var_id, .. },
                                        required: _,
                                        type_annotated,
                                    } => {
                                        working_set.set_variable_type(var_id.expect("internal error: all custom parameters must have var_ids"), syntax_shape.to_type());
                                        *shape = syntax_shape;
                                        *type_annotated = true;
                                    }
                                    Arg::RestPositional(PositionalArg {
                                        shape, var_id, ..
                                    }) => {
                                        working_set.set_variable_type(var_id.expect("internal error: all custom parameters must have var_ids"), Type::List(Box::new(syntax_shape.to_type())));
                                        *shape = syntax_shape;
                                    }
                                    Arg::Flag {
                                        flag: Flag { arg, var_id, .. },
                                        type_annotated,
                                    } => {
                                        working_set.set_variable_type(var_id.expect("internal error: all custom parameters must have var_ids"), syntax_shape.to_type());
                                        if syntax_shape == SyntaxShape::Boolean {
                                            working_set.error(ParseError::LabeledError(
                                                "Type annotations are not allowed for boolean switches.".to_string(),
                                                "Remove the `: bool` type annotation.".to_string(),
                                                span,
                                            ));
                                        }
                                        *arg = Some(syntax_shape);
                                        *type_annotated = true;
                                    }
                                }
                            }
                            parse_mode = ParseMode::Arg;
                        }
                        ParseMode::DefaultValue => {
                            if let Some(last) = args.last_mut() {
                                let expression = parse_value(working_set, span, &SyntaxShape::Any);

                                //TODO check if we're replacing a custom parameter already
                                match last {
                                    Arg::Positional {
                                        arg:
                                            PositionalArg {
                                                shape,
                                                var_id,
                                                default_value,
                                                ..
                                            },
                                        required,
                                        type_annotated,
                                    } => {
                                        let var_id = var_id.expect("internal error: all custom parameters must have var_ids");
                                        let var_type = &working_set.get_variable(var_id).ty;
                                        match var_type {
                                            Type::Any => {
                                                if !*type_annotated {
                                                    working_set.set_variable_type(
                                                        var_id,
                                                        expression.ty.clone(),
                                                    );
                                                }
                                            }
                                            _ => {
                                                if !type_compatible(var_type, &expression.ty) {
                                                    working_set.error(
                                                        ParseError::AssignmentMismatch(
                                                            "Default value wrong type".into(),
                                                            format!(
                                                            "expected default value to be `{var_type}`"
                                                        ),
                                                            expression.span,
                                                        ),
                                                    )
                                                }
                                            }
                                        }

                                        *default_value = if let Ok(constant) =
                                            eval_constant(working_set, &expression)
                                        {
                                            Some(constant)
                                        } else {
                                            working_set.error(ParseError::NonConstantDefaultValue(
                                                expression.span,
                                            ));
                                            None
                                        };

                                        if !*type_annotated {
                                            *shape = expression.ty.to_shape();
                                        }
                                        *required = false;
                                    }
                                    Arg::RestPositional(..) => {
                                        working_set.error(ParseError::AssignmentMismatch(
                                            "Rest parameter was given a default value".into(),
                                            "can't have default value".into(),
                                            expression.span,
                                        ))
                                    }
                                    Arg::Flag {
                                        flag:
                                            Flag {
                                                arg,
                                                var_id,
                                                default_value,
                                                ..
                                            },
                                        type_annotated,
                                    } => {
                                        let expression_span = expression.span;

                                        *default_value = if let Ok(value) =
                                            eval_constant(working_set, &expression)
                                        {
                                            Some(value)
                                        } else {
                                            working_set.error(ParseError::NonConstantDefaultValue(
                                                expression_span,
                                            ));
                                            None
                                        };

                                        let var_id = var_id.expect("internal error: all custom parameters must have var_ids");
                                        let var_type = &working_set.get_variable(var_id).ty;
                                        let expression_ty = expression.ty.clone();

                                        // Flags with no TypeMode are just present/not-present switches
                                        // in the case, `var_type` is any.
                                        match var_type {
                                            Type::Any => {
                                                if !*type_annotated {
                                                    *arg = Some(expression_ty.to_shape());
                                                    working_set
                                                        .set_variable_type(var_id, expression_ty);
                                                }
                                            }
                                            t => {
                                                if !type_compatible(t, &expression_ty) {
                                                    working_set.error(
                                                        ParseError::AssignmentMismatch(
                                                            "Default value is the wrong type"
                                                                .into(),
                                                            format!(
                                                            "expected default value to be `{t}`"
                                                                ),
                                                            expression_span,
                                                        ),
                                                    )
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            parse_mode = ParseMode::Arg;
                        }
                    }
                }
            }
            Token {
                contents: crate::TokenContents::Comment,
                span,
            } => {
                let contents = working_set.get_span_contents(Span::new(span.start + 1, span.end));

                let mut contents = String::from_utf8_lossy(contents).to_string();
                contents = contents.trim().into();

                if let Some(last) = args.last_mut() {
                    match last {
                        Arg::Flag { flag, .. } => {
                            if !flag.desc.is_empty() {
                                flag.desc.push('\n');
                            }
                            flag.desc.push_str(&contents);
                        }
                        Arg::Positional {
                            arg: positional, ..
                        } => {
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
            Arg::Positional {
                arg: positional,
                required,
                ..
            } => {
                if required {
                    if !sig.optional_positional.is_empty() {
                        working_set.error(ParseError::RequiredAfterOptional(
                            positional.name.clone(),
                            span,
                        ))
                    }
                    sig.required_positional.push(positional)
                } else {
                    sig.optional_positional.push(positional)
                }
            }
            Arg::Flag { flag, .. } => sig.named.push(flag),
            Arg::RestPositional(positional) => {
                if positional.name.is_empty() {
                    working_set.error(ParseError::RestNeedsName(span))
                } else if sig.rest_positional.is_none() {
                    sig.rest_positional = Some(PositionalArg {
                        name: positional.name,
                        ..positional
                    })
                } else {
                    // Too many rest params
                    working_set.error(ParseError::MultipleRestParams(span))
                }
            }
        }
    }

    Box::new(sig)
}

pub fn parse_list_expression(
    working_set: &mut StateWorkingSet,
    span: Span,
    element_shape: &SyntaxShape,
) -> Expression {
    let bytes = working_set.get_span_contents(span);

    let mut start = span.start;
    let mut end = span.end;

    if bytes.starts_with(b"[") {
        start += 1;
    }
    if bytes.ends_with(b"]") {
        end -= 1;
    } else {
        working_set.error(ParseError::Unclosed("]".into(), Span::new(end, end)));
    }

    let inner_span = Span::new(start, end);
    let source = working_set.get_span_contents(inner_span);

    let (output, err) = lex(source, inner_span.start, &[b'\n', b'\r', b','], &[], true);
    if let Some(err) = err {
        working_set.error(err)
    }

    let (mut output, err) = lite_parse(&output);
    if let Some(err) = err {
        working_set.error(err)
    }

    let mut args = vec![];

    let mut contained_type: Option<Type> = None;

    if !output.block.is_empty() {
        for mut command in output.block.remove(0).commands {
            let mut spans_idx = 0;

            while spans_idx < command.parts.len() {
                let curr_span = command.parts[spans_idx];
                let curr_tok = working_set.get_span_contents(curr_span);
                let (arg, ty) = if curr_tok.starts_with(b"...")
                    && curr_tok.len() > 3
                    && (curr_tok[3] == b'$' || curr_tok[3] == b'[' || curr_tok[3] == b'(')
                {
                    // Parse the spread operator
                    // Remove "..." before parsing argument to spread operator
                    command.parts[spans_idx] = Span::new(curr_span.start + 3, curr_span.end);
                    let spread_arg = parse_multispan_value(
                        working_set,
                        &command.parts,
                        &mut spans_idx,
                        &SyntaxShape::List(Box::new(element_shape.clone())),
                    );
                    let elem_ty = match &spread_arg.ty {
                        Type::List(elem_ty) => *elem_ty.clone(),
                        _ => Type::Any,
                    };
                    let span = Span::new(curr_span.start, curr_span.start + 3);
                    (ListItem::Spread(span, spread_arg), elem_ty)
                } else {
                    let arg = parse_multispan_value(
                        working_set,
                        &command.parts,
                        &mut spans_idx,
                        element_shape,
                    );
                    let ty = arg.ty.clone();
                    (ListItem::Item(arg), ty)
                };

                if let Some(ref ctype) = contained_type {
                    if *ctype != ty {
                        contained_type = Some(Type::Any);
                    }
                } else {
                    contained_type = Some(ty);
                }

                args.push(arg);

                spans_idx += 1;
            }
        }
    }

    Expression {
        expr: Expr::List(args),
        span,
        ty: Type::List(Box::new(if let Some(ty) = contained_type {
            ty
        } else {
            Type::Any
        })),
        custom_completion: None,
    }
}

fn parse_table_row(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> Result<(Vec<Expression>, Span), Span> {
    let list = parse_list_expression(working_set, span, &SyntaxShape::Any);
    let Expression {
        expr: Expr::List(list),
        span,
        ..
    } = list
    else {
        unreachable!("the item must be a list")
    };

    list.into_iter()
        .map(|item| match item {
            ListItem::Item(expr) => Ok(expr),
            ListItem::Spread(_, spread) => Err(spread.span),
        })
        .collect::<Result<_, _>>()
        .map(|exprs| (exprs, span))
}

fn parse_table_expression(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let bytes = working_set.get_span_contents(span);
    let inner_span = {
        let start = if bytes.starts_with(b"[") {
            span.start + 1
        } else {
            span.start
        };

        let end = if bytes.ends_with(b"]") {
            span.end - 1
        } else {
            let end = span.end;
            working_set.error(ParseError::Unclosed("]".into(), Span::new(end, end)));
            span.end
        };

        Span::new(start, end)
    };

    let source = working_set.get_span_contents(inner_span);
    let (tokens, err) = lex(source, inner_span.start, &[b'\n', b'\r', b','], &[], true);
    if let Some(err) = err {
        working_set.error(err);
    }

    // Check that we have all arguments first, before trying to parse the first
    // in order to avoid exponential parsing time
    let [first, second, rest @ ..] = &tokens[..] else {
        return parse_list_expression(working_set, span, &SyntaxShape::Any);
    };
    if !working_set.get_span_contents(first.span).starts_with(b"[")
        || second.contents != TokenContents::Semicolon
        || rest.is_empty()
    {
        return parse_list_expression(working_set, span, &SyntaxShape::Any);
    };
    let head = parse_table_row(working_set, first.span);

    let errors = working_set.parse_errors.len();

    let (head, rows) = match head {
        Ok((head, _)) => {
            let rows = rest
                .iter()
                .filter_map(|it| {
                    use std::cmp::Ordering;

                    match working_set.get_span_contents(it.span) {
                        b"," => None,
                        text if !text.starts_with(b"[") => {
                            let err = ParseError::LabeledErrorWithHelp {
                                error: String::from("Table item not list"),
                                label: String::from("not a list"),
                                span: it.span,
                                help: String::from("All table items must be lists"),
                            };
                            working_set.error(err);
                            None
                        }
                        _ => match parse_table_row(working_set, it.span) {
                            Ok((list, span)) => {
                                match list.len().cmp(&head.len()) {
                                    Ordering::Less => {
                                        let err = ParseError::MissingColumns(head.len(), span);
                                        working_set.error(err);
                                    }
                                    Ordering::Greater => {
                                        let span = {
                                            let start = list[head.len()].span.start;
                                            let end = span.end;
                                            Span::new(start, end)
                                        };
                                        let err = ParseError::ExtraColumns(head.len(), span);
                                        working_set.error(err);
                                    }
                                    Ordering::Equal => {}
                                }
                                Some(list)
                            }
                            Err(span) => {
                                let err = ParseError::LabeledError(
                                    String::from("Cannot spread in a table row"),
                                    String::from("invalid spread here"),
                                    span,
                                );
                                working_set.error(err);
                                None
                            }
                        },
                    }
                })
                .collect();

            (head, rows)
        }
        Err(span) => {
            let err = ParseError::LabeledError(
                String::from("Cannot spread in a table row"),
                String::from("invalid spread here"),
                span,
            );
            working_set.error(err);
            (Vec::new(), Vec::new())
        }
    };

    let ty = if working_set.parse_errors.len() == errors {
        let (ty, errs) = table_type(&head, &rows);
        working_set.parse_errors.extend(errs);
        ty
    } else {
        Type::table()
    };

    let table = Table {
        columns: head.into(),
        rows: rows.into_iter().map(Into::into).collect(),
    };

    Expression {
        expr: Expr::Table(table),
        span,
        ty,
        custom_completion: None,
    }
}

fn table_type(head: &[Expression], rows: &[Vec<Expression>]) -> (Type, Vec<ParseError>) {
    let mut errors = vec![];
    let mut rows = rows.to_vec();
    let mut mk_ty = || -> Type {
        rows.iter_mut()
            .map(|row| row.pop().map(|x| x.ty).unwrap_or_default())
            .reduce(|acc, ty| -> Type {
                if type_compatible(&acc, &ty) {
                    ty
                } else {
                    Type::Any
                }
            })
            .unwrap_or_default()
    };

    let mk_error = |span| ParseError::LabeledErrorWithHelp {
        error: "Table column name not string".into(),
        label: "must be a string".into(),
        help: "Table column names should be able to be converted into strings".into(),
        span,
    };

    let mut ty = head
        .iter()
        .rev()
        .map(|expr| {
            if let Some(str) = expr.as_string() {
                str
            } else {
                errors.push(mk_error(expr.span));
                String::from("{ column }")
            }
        })
        .map(|title| (title, mk_ty()))
        .collect_vec();

    ty.reverse();

    (Type::Table(ty.into()), errors)
}

pub fn parse_block_expression(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: block expression");

    let bytes = working_set.get_span_contents(span);

    let mut start = span.start;
    let mut end = span.end;

    if bytes.starts_with(b"{") {
        start += 1;
    } else {
        working_set.error(ParseError::Expected("block", span));
        return garbage(span);
    }
    if bytes.ends_with(b"}") {
        end -= 1;
    } else {
        working_set.error(ParseError::Unclosed("}".into(), Span::new(end, end)));
    }

    let inner_span = Span::new(start, end);

    let source = working_set.get_span_contents(inner_span);

    let (output, err) = lex(source, start, &[], &[], false);
    if let Some(err) = err {
        working_set.error(err);
    }

    working_set.enter_scope();

    // Check to see if we have parameters
    let (signature, amt_to_skip): (Option<(Box<Signature>, Span)>, usize) = match output.first() {
        Some(Token {
            contents: TokenContents::Pipe,
            span,
        }) => {
            working_set.error(ParseError::Expected("block but found closure", *span));
            (None, 0)
        }
        _ => (None, 0),
    };

    let mut output = parse_block(working_set, &output[amt_to_skip..], span, false, false);

    if let Some(signature) = signature {
        output.signature = signature.0;
    }

    output.span = Some(span);

    working_set.exit_scope();

    let block_id = working_set.add_block(Arc::new(output));

    Expression {
        expr: Expr::Block(block_id),
        span,
        ty: Type::Block,
        custom_completion: None,
    }
}

pub fn parse_match_block_expression(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let bytes = working_set.get_span_contents(span);

    let mut start = span.start;
    let mut end = span.end;

    if bytes.starts_with(b"{") {
        start += 1;
    } else {
        working_set.error(ParseError::Expected("closure", span));
        return garbage(span);
    }
    if bytes.ends_with(b"}") {
        end -= 1;
    } else {
        working_set.error(ParseError::Unclosed("}".into(), Span::new(end, end)));
    }

    let inner_span = Span::new(start, end);

    let source = working_set.get_span_contents(inner_span);

    let (output, err) = lex(source, start, &[b' ', b'\r', b'\n', b',', b'|'], &[], true);
    if let Some(err) = err {
        working_set.error(err);
    }

    let mut position = 0;

    let mut output_matches = vec![];

    while position < output.len() {
        // Each match gets its own scope

        working_set.enter_scope();

        // First parse the pattern
        let mut pattern = parse_pattern(working_set, output[position].span);

        position += 1;

        if position >= output.len() {
            working_set.error(ParseError::Mismatch(
                "=>".into(),
                "end of input".into(),
                Span::new(output[position - 1].span.end, output[position - 1].span.end),
            ));

            working_set.exit_scope();
            break;
        }

        let mut connector = working_set.get_span_contents(output[position].span);

        // Multiple patterns connected by '|'
        if connector == b"|" && position < output.len() {
            let mut or_pattern = vec![pattern];

            while connector == b"|" && position < output.len() {
                connector = b"";

                position += 1;

                if position >= output.len() {
                    working_set.error(ParseError::Mismatch(
                        "pattern".into(),
                        "end of input".into(),
                        Span::new(output[position - 1].span.end, output[position - 1].span.end),
                    ));

                    working_set.exit_scope();
                    break;
                }

                let pattern = parse_pattern(working_set, output[position].span);
                or_pattern.push(pattern);

                position += 1;
                if position >= output.len() {
                    working_set.error(ParseError::Mismatch(
                        "=>".into(),
                        "end of input".into(),
                        Span::new(output[position - 1].span.end, output[position - 1].span.end),
                    ));

                    working_set.exit_scope();
                    break;
                } else {
                    connector = working_set.get_span_contents(output[position].span);
                }
            }

            let start = or_pattern
                .first()
                .expect("internal error: unexpected state of or-pattern")
                .span
                .start;
            let end = or_pattern
                .last()
                .expect("internal error: unexpected state of or-pattern")
                .span
                .end;

            pattern = MatchPattern {
                pattern: Pattern::Or(or_pattern),
                guard: None,
                span: Span::new(start, end),
            }
        // A match guard
        } else if connector == b"if" {
            let if_end = {
                let end = output[position].span.end;
                Span::new(end, end)
            };

            position += 1;

            let mk_err = || ParseError::LabeledErrorWithHelp {
                error: "Match guard without an expression".into(),
                label: "expected an expression".into(),
                help: "The `if` keyword must be followed with an expression".into(),
                span: if_end,
            };

            if output.get(position).is_none() {
                working_set.error(mk_err());
                return garbage(span);
            };

            let (tokens, found) = if let Some((pos, _)) = output[position..]
                .iter()
                .find_position(|t| working_set.get_span_contents(t.span) == b"=>")
            {
                if position + pos == position {
                    working_set.error(mk_err());
                    return garbage(span);
                }

                (&output[position..position + pos], true)
            } else {
                (&output[position..], false)
            };

            let mut start = 0;
            let guard = parse_multispan_value(
                working_set,
                &tokens.iter().map(|tok| tok.span).collect_vec(),
                &mut start,
                &SyntaxShape::MathExpression,
            );

            pattern.guard = Some(guard);
            position += if found { start + 1 } else { start };
            connector = working_set.get_span_contents(output[position].span);
        }
        // Then the `=>` arrow
        if connector != b"=>" {
            working_set.error(ParseError::Mismatch(
                "=>".into(),
                "end of input".into(),
                Span::new(output[position - 1].span.end, output[position - 1].span.end),
            ));
        } else {
            position += 1;
        }

        // Finally, the value/expression/block that we will run to produce the result
        if position >= output.len() {
            working_set.error(ParseError::Mismatch(
                "match result".into(),
                "end of input".into(),
                Span::new(output[position - 1].span.end, output[position - 1].span.end),
            ));

            working_set.exit_scope();
            break;
        }

        let result = parse_multispan_value(
            working_set,
            &[output[position].span],
            &mut 0,
            &SyntaxShape::OneOf(vec![SyntaxShape::Block, SyntaxShape::Expression]),
        );
        position += 1;
        working_set.exit_scope();

        output_matches.push((pattern, result));
    }

    Expression {
        expr: Expr::MatchBlock(output_matches),
        span,
        ty: Type::Any,
        custom_completion: None,
    }
}

pub fn parse_closure_expression(
    working_set: &mut StateWorkingSet,
    shape: &SyntaxShape,
    span: Span,
) -> Expression {
    trace!("parsing: closure expression");

    let bytes = working_set.get_span_contents(span);

    let mut start = span.start;
    let mut end = span.end;

    if bytes.starts_with(b"{") {
        start += 1;
    } else {
        working_set.error(ParseError::Expected("closure", span));
        return garbage(span);
    }
    if bytes.ends_with(b"}") {
        end -= 1;
    } else {
        working_set.error(ParseError::Unclosed("}".into(), Span::new(end, end)));
    }

    let inner_span = Span::new(start, end);

    let source = working_set.get_span_contents(inner_span);

    let (output, err) = lex(source, start, &[], &[], false);
    if let Some(err) = err {
        working_set.error(err);
    }

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

            let signature_span = Span::new(start_point, end_point);
            let signature = parse_signature_helper(working_set, signature_span);

            (Some((signature, signature_span)), amt_to_skip)
        }
        Some(Token {
            contents: TokenContents::PipePipe,
            span,
        }) => (
            Some((Box::new(Signature::new("closure".to_string())), *span)),
            1,
        ),
        _ => (None, 0),
    };

    // TODO: Finish this
    if let SyntaxShape::Closure(Some(v)) = shape {
        if let Some((sig, sig_span)) = &signature {
            if sig.num_positionals() > v.len() {
                working_set.error(ParseError::ExpectedWithStringMsg(
                    format!(
                        "{} closure parameter{}",
                        v.len(),
                        if v.len() > 1 { "s" } else { "" }
                    ),
                    *sig_span,
                ));
            }

            for (expected, PositionalArg { name, shape, .. }) in
                v.iter().zip(sig.required_positional.iter())
            {
                if expected != shape && *shape != SyntaxShape::Any {
                    working_set.error(ParseError::ParameterMismatchType(
                        name.to_owned(),
                        expected.to_string(),
                        shape.to_string(),
                        *sig_span,
                    ));
                }
            }
        }
    }

    let mut output = parse_block(working_set, &output[amt_to_skip..], span, false, false);

    if let Some(signature) = signature {
        output.signature = signature.0;
    }

    output.span = Some(span);

    working_set.exit_scope();

    let block_id = working_set.add_block(Arc::new(output));

    Expression {
        expr: Expr::Closure(block_id),
        span,
        ty: Type::Closure,
        custom_completion: None,
    }
}

pub fn parse_value(
    working_set: &mut StateWorkingSet,
    span: Span,
    shape: &SyntaxShape,
) -> Expression {
    trace!("parsing: value: {}", shape);

    let bytes = working_set.get_span_contents(span);

    if bytes.is_empty() {
        working_set.error(ParseError::IncompleteParser(span));
        return garbage(span);
    }

    // Check for reserved keyword values
    match bytes {
        b"true" => {
            if matches!(shape, SyntaxShape::Boolean) || matches!(shape, SyntaxShape::Any) {
                return Expression {
                    expr: Expr::Bool(true),
                    span,
                    ty: Type::Bool,
                    custom_completion: None,
                };
            } else {
                working_set.error(ParseError::Expected("non-boolean value", span));
                return Expression::garbage(span);
            }
        }
        b"false" => {
            if matches!(shape, SyntaxShape::Boolean) || matches!(shape, SyntaxShape::Any) {
                return Expression {
                    expr: Expr::Bool(false),
                    span,
                    ty: Type::Bool,
                    custom_completion: None,
                };
            } else {
                working_set.error(ParseError::Expected("non-boolean value", span));
                return Expression::garbage(span);
            }
        }
        b"null" => {
            return Expression {
                expr: Expr::Nothing,
                span,
                ty: Type::Nothing,
                custom_completion: None,
            };
        }
        b"-inf" | b"inf" | b"NaN" => {
            return parse_float(working_set, span);
        }
        _ => {}
    }

    match bytes[0] {
        b'$' => return parse_dollar_expr(working_set, span),
        b'(' => return parse_paren_expr(working_set, span, shape),
        b'{' => return parse_brace_expr(working_set, span, shape),
        b'[' => match shape {
            SyntaxShape::Any
            | SyntaxShape::List(_)
            | SyntaxShape::Table(_)
            | SyntaxShape::Signature
            | SyntaxShape::Filepath
            | SyntaxShape::String
            | SyntaxShape::GlobPattern => {}
            _ => {
                working_set.error(ParseError::Expected("non-[] value", span));
                return Expression::garbage(span);
            }
        },
        b'r' if bytes.len() > 1 && bytes[1] == b'#' => {
            return parse_raw_string(working_set, span);
        }
        _ => {}
    }

    match shape {
        SyntaxShape::CompleterWrapper(shape, custom_completion) => {
            let mut expression = parse_value(working_set, span, shape);
            expression.custom_completion = Some(*custom_completion);
            expression
        }
        SyntaxShape::Number => parse_number(working_set, span),
        SyntaxShape::Float => parse_float(working_set, span),
        SyntaxShape::Int => parse_int(working_set, span),
        SyntaxShape::Duration => parse_duration(working_set, span),
        SyntaxShape::DateTime => parse_datetime(working_set, span),
        SyntaxShape::Filesize => parse_filesize(working_set, span),
        SyntaxShape::Range => parse_range(working_set, span),
        SyntaxShape::Filepath => parse_filepath(working_set, span),
        SyntaxShape::Directory => parse_directory(working_set, span),
        SyntaxShape::GlobPattern => parse_glob_pattern(working_set, span),
        SyntaxShape::String => parse_string(working_set, span),
        SyntaxShape::Binary => parse_binary(working_set, span),
        SyntaxShape::Signature => {
            if bytes.starts_with(b"[") {
                parse_signature(working_set, span)
            } else {
                working_set.error(ParseError::Expected("signature", span));

                Expression::garbage(span)
            }
        }
        SyntaxShape::List(elem) => {
            if bytes.starts_with(b"[") {
                parse_list_expression(working_set, span, elem)
            } else {
                working_set.error(ParseError::Expected("list", span));

                Expression::garbage(span)
            }
        }
        SyntaxShape::Table(_) => {
            if bytes.starts_with(b"[") {
                parse_table_expression(working_set, span)
            } else {
                working_set.error(ParseError::Expected("table", span));

                Expression::garbage(span)
            }
        }
        SyntaxShape::CellPath => parse_simple_cell_path(working_set, span),
        SyntaxShape::Boolean => {
            // Redundant, though we catch bad boolean parses here
            if bytes == b"true" || bytes == b"false" {
                Expression {
                    expr: Expr::Bool(true),
                    span,
                    ty: Type::Bool,
                    custom_completion: None,
                }
            } else {
                working_set.error(ParseError::Expected("bool", span));

                Expression::garbage(span)
            }
        }

        // Be sure to return ParseError::Expected(..) if invoked for one of these shapes, but lex
        // stream doesn't start with '{'} -- parsing in SyntaxShape::Any arm depends on this error variant.
        SyntaxShape::Block | SyntaxShape::Closure(..) | SyntaxShape::Record(_) => {
            working_set.error(ParseError::Expected("block, closure or record", span));

            Expression::garbage(span)
        }

        SyntaxShape::Any => {
            if bytes.starts_with(b"[") {
                //parse_value(working_set, span, &SyntaxShape::Table)
                parse_full_cell_path(working_set, None, span)
            } else {
                let shapes = [
                    SyntaxShape::Binary,
                    SyntaxShape::Filesize,
                    SyntaxShape::Duration,
                    SyntaxShape::Range,
                    SyntaxShape::DateTime,
                    SyntaxShape::Int,
                    SyntaxShape::Number,
                    SyntaxShape::String,
                ];
                for shape in shapes.iter() {
                    let starting_error_count = working_set.parse_errors.len();

                    let s = parse_value(working_set, span, shape);

                    if starting_error_count == working_set.parse_errors.len() {
                        return s;
                    } else {
                        match working_set.parse_errors.get(starting_error_count) {
                            Some(
                                ParseError::Expected(_, _)
                                | ParseError::ExpectedWithStringMsg(_, _),
                            ) => {
                                working_set.parse_errors.truncate(starting_error_count);
                                continue;
                            }
                            _ => {
                                return s;
                            }
                        }
                    }
                }
                working_set.error(ParseError::Expected("any shape", span));
                garbage(span)
            }
        }
        x => {
            working_set.error(ParseError::ExpectedWithStringMsg(
                x.to_type().to_string(),
                span,
            ));
            garbage(span)
        }
    }
}

pub fn parse_operator(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let contents = working_set.get_span_contents(span);

    let operator = match contents {
        b"=" => Operator::Assignment(Assignment::Assign),
        b"+=" => Operator::Assignment(Assignment::PlusAssign),
        b"++=" => Operator::Assignment(Assignment::AppendAssign),
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
        b"and" => Operator::Boolean(Boolean::And),
        b"or" => Operator::Boolean(Boolean::Or),
        b"xor" => Operator::Boolean(Boolean::Xor),
        b"**" => Operator::Math(Math::Pow),
        // WARNING: not actual operators below! Error handling only
        pow @ (b"^" | b"pow") => {
            working_set.error(ParseError::UnknownOperator(
                match pow {
                    b"^" => "^",
                    b"pow" => "pow",
                    _ => unreachable!(),
                },
                "Use '**' for exponentiation or 'bit-xor' for bitwise XOR.",
                span,
            ));
            return garbage(span);
        }
        equality @ (b"is" | b"===") => {
            working_set.error(ParseError::UnknownOperator(
                match equality {
                    b"is" => "is",
                    b"===" => "===",
                    _ => unreachable!(),
                },
                "Did you mean '=='?",
                span,
            ));
            return garbage(span);
        }
        b"contains" => {
            working_set.error(ParseError::UnknownOperator(
                "contains",
                "Did you mean '$string =~ $pattern' or '$element in $container'?",
                span,
            ));
            return garbage(span);
        }
        b"%" => {
            working_set.error(ParseError::UnknownOperator(
                "%",
                "Did you mean 'mod'?",
                span,
            ));
            return garbage(span);
        }
        b"&" => {
            working_set.error(ParseError::UnknownOperator(
                "&",
                "Did you mean 'bit-and'?",
                span,
            ));
            return garbage(span);
        }
        b"<<" => {
            working_set.error(ParseError::UnknownOperator(
                "<<",
                "Did you mean 'bit-shl'?",
                span,
            ));
            return garbage(span);
        }
        b">>" => {
            working_set.error(ParseError::UnknownOperator(
                ">>",
                "Did you mean 'bit-shr'?",
                span,
            ));
            return garbage(span);
        }
        bits @ (b"bits-and" | b"bits-xor" | b"bits-or" | b"bits-shl" | b"bits-shr") => {
            working_set.error(ParseError::UnknownOperator(
                match bits {
                    b"bits-and" => "bits-and",
                    b"bits-xor" => "bits-xor",
                    b"bits-or" => "bits-or",
                    b"bits-shl" => "bits-shl",
                    b"bits-shr" => "bits-shr",
                    _ => unreachable!(),
                },
                match bits {
                    b"bits-and" => "Did you mean 'bit-and'?",
                    b"bits-xor" => "Did you mean 'bit-xor'?",
                    b"bits-or" => "Did you mean 'bit-or'?",
                    b"bits-shl" => "Did you mean 'bit-shl'?",
                    b"bits-shr" => "Did you mean 'bit-shr'?",
                    _ => unreachable!(),
                },
                span,
            ));
            return garbage(span);
        }
        _ => {
            working_set.error(ParseError::Expected("operator", span));
            return garbage(span);
        }
    };

    Expression {
        expr: Expr::Operator(operator),
        span,
        ty: Type::Any,
        custom_completion: None,
    }
}

pub fn parse_math_expression(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    lhs_row_var_id: Option<VarId>,
) -> Expression {
    trace!("parsing: math expression");

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

    let first_span = working_set.get_span_contents(spans[0]);

    let mut not_start_spans = vec![];

    if first_span == b"if" || first_span == b"match" {
        // If expression
        if spans.len() > 1 {
            return parse_call(working_set, spans, spans[0]);
        } else {
            working_set.error(ParseError::Expected(
                "expression",
                Span::new(spans[0].end, spans[0].end),
            ));
            return garbage(spans[0]);
        }
    } else if first_span == b"not" {
        not_start_spans.push(spans[idx].start);
        idx += 1;
        while idx < spans.len() {
            let next_value = working_set.get_span_contents(spans[idx]);

            if next_value == b"not" {
                not_start_spans.push(spans[idx].start);
                idx += 1;
            } else {
                break;
            }
        }

        if idx == spans.len() {
            working_set.error(ParseError::Expected(
                "expression",
                Span::new(spans[idx - 1].end, spans[idx - 1].end),
            ));
            return garbage(spans[idx - 1]);
        }
    }

    let mut lhs = parse_value(working_set, spans[idx], &SyntaxShape::Any);

    for not_start_span in not_start_spans.iter().rev() {
        lhs = Expression {
            expr: Expr::UnaryNot(Box::new(lhs)),
            span: Span::new(*not_start_span, spans[idx].end),
            ty: Type::Bool,
            custom_completion: None,
        };
    }
    not_start_spans.clear();

    idx += 1;

    if idx >= spans.len() {
        // We already found the one part of our expression, so let's expand
        if let Some(row_var_id) = lhs_row_var_id {
            expand_to_cell_path(working_set, &mut lhs, row_var_id);
        }
    }

    expr_stack.push(lhs);

    while idx < spans.len() {
        let op = parse_operator(working_set, spans[idx]);

        let op_prec = op.precedence();

        idx += 1;

        if idx == spans.len() {
            // Handle broken math expr `1 +` etc
            working_set.error(ParseError::IncompleteMathExpression(spans[idx - 1]));

            expr_stack.push(Expression::garbage(spans[idx - 1]));
            expr_stack.push(Expression::garbage(spans[idx - 1]));

            break;
        }

        let content = working_set.get_span_contents(spans[idx]);
        // allow `if` to be a special value for assignment.

        if content == b"if" || content == b"match" {
            let rhs = parse_call(working_set, &spans[idx..], spans[0]);
            expr_stack.push(op);
            expr_stack.push(rhs);
            break;
        } else if content == b"not" {
            not_start_spans.push(spans[idx].start);
            idx += 1;
            while idx < spans.len() {
                let next_value = working_set.get_span_contents(spans[idx]);

                if next_value == b"not" {
                    not_start_spans.push(spans[idx].start);
                    idx += 1;
                } else {
                    break;
                }
            }

            if idx == spans.len() {
                working_set.error(ParseError::Expected(
                    "expression",
                    Span::new(spans[idx - 1].end, spans[idx - 1].end),
                ));
                return garbage(spans[idx - 1]);
            }
        }
        let mut rhs = parse_value(working_set, spans[idx], &SyntaxShape::Any);

        for not_start_span in not_start_spans.iter().rev() {
            rhs = Expression {
                expr: Expr::UnaryNot(Box::new(rhs)),
                span: Span::new(*not_start_span, spans[idx].end),
                ty: Type::Bool,
                custom_completion: None,
            };
        }
        not_start_spans.clear();

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
                expand_to_cell_path(working_set, &mut lhs, row_var_id);
            }

            let (result_ty, err) = math_result_type(working_set, &mut lhs, &mut op, &mut rhs);
            if let Some(err) = err {
                working_set.error(err);
            }

            let op_span = Span::append(lhs.span, rhs.span);
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
            expand_to_cell_path(working_set, &mut lhs, row_var_id);
        }

        let (result_ty, err) = math_result_type(working_set, &mut lhs, &mut op, &mut rhs);
        if let Some(err) = err {
            working_set.error(err)
        }

        let binary_op_span = Span::append(lhs.span, rhs.span);
        expr_stack.push(Expression {
            expr: Expr::BinaryOp(Box::new(lhs), Box::new(op), Box::new(rhs)),
            span: binary_op_span,
            ty: result_ty,
            custom_completion: None,
        });
    }

    expr_stack
        .pop()
        .expect("internal error: expression stack empty")
}

pub fn parse_expression(working_set: &mut StateWorkingSet, spans: &[Span]) -> Expression {
    trace!("parsing: expression");

    let mut pos = 0;
    let mut shorthand = vec![];

    while pos < spans.len() {
        // Check if there is any environment shorthand
        let name = working_set.get_span_contents(spans[pos]);

        let split = name.splitn(2, |x| *x == b'=');
        let split: Vec<_> = split.collect();
        if !name.starts_with(b"^")
            && split.len() == 2
            && !split[0].is_empty()
            && !split[0].ends_with(b"..")
        // was range op ..=
        {
            let point = split[0].len() + 1;

            let starting_error_count = working_set.parse_errors.len();

            let lhs = parse_string_strict(
                working_set,
                Span::new(spans[pos].start, spans[pos].start + point - 1),
            );
            let rhs = if spans[pos].start + point < spans[pos].end {
                let rhs_span = Span::new(spans[pos].start + point, spans[pos].end);

                if working_set.get_span_contents(rhs_span).starts_with(b"$") {
                    parse_dollar_expr(working_set, rhs_span)
                } else {
                    parse_string_strict(working_set, rhs_span)
                }
            } else {
                Expression {
                    expr: Expr::String(String::new()),
                    span: Span::unknown(),
                    ty: Type::Nothing,
                    custom_completion: None,
                }
            };

            if starting_error_count == working_set.parse_errors.len() {
                shorthand.push((lhs, rhs));
                pos += 1;
            } else {
                working_set.parse_errors.truncate(starting_error_count);
                break;
            }
        } else {
            break;
        }
    }

    if pos == spans.len() {
        working_set.error(ParseError::UnknownCommand(spans[0]));
        return garbage(Span::concat(spans));
    }

    let output = if is_math_expression_like(working_set, spans[pos]) {
        parse_math_expression(working_set, &spans[pos..], None)
    } else {
        let bytes = working_set.get_span_contents(spans[pos]).to_vec();

        // For now, check for special parses of certain keywords
        match bytes.as_slice() {
            b"def" | b"extern" | b"for" | b"module" | b"use" | b"source" | b"alias" | b"export"
            | b"hide" => {
                working_set.error(ParseError::BuiltinCommandInPipeline(
                    String::from_utf8(bytes)
                        .expect("builtin commands bytes should be able to convert to string"),
                    spans[0],
                ));

                parse_call(working_set, &spans[pos..], spans[0])
            }
            b"let" | b"const" | b"mut" => {
                working_set.error(ParseError::AssignInPipeline(
                    String::from_utf8(bytes)
                        .expect("builtin commands bytes should be able to convert to string"),
                    String::from_utf8_lossy(match spans.len() {
                        1..=3 => b"value",
                        _ => working_set.get_span_contents(spans[3]),
                    })
                    .to_string(),
                    String::from_utf8_lossy(match spans.len() {
                        1 => b"variable",
                        _ => working_set.get_span_contents(spans[1]),
                    })
                    .to_string(),
                    spans[0],
                ));
                parse_call(working_set, &spans[pos..], spans[0])
            }
            b"overlay" => {
                if spans.len() > 1 && working_set.get_span_contents(spans[1]) == b"list" {
                    // whitelist 'overlay list'
                    parse_call(working_set, &spans[pos..], spans[0])
                } else {
                    working_set.error(ParseError::BuiltinCommandInPipeline(
                        "overlay".into(),
                        spans[0],
                    ));

                    parse_call(working_set, &spans[pos..], spans[0])
                }
            }
            b"where" => parse_where_expr(working_set, &spans[pos..]),
            #[cfg(feature = "plugin")]
            b"register" => {
                working_set.error(ParseError::BuiltinCommandInPipeline(
                    "register".into(),
                    spans[0],
                ));

                parse_call(working_set, &spans[pos..], spans[0])
            }
            #[cfg(feature = "plugin")]
            b"plugin" => {
                if spans.len() > 1 && working_set.get_span_contents(spans[1]) == b"use" {
                    // only 'plugin use' is banned
                    working_set.error(ParseError::BuiltinCommandInPipeline(
                        "plugin use".into(),
                        spans[0],
                    ));
                }

                parse_call(working_set, &spans[pos..], spans[0])
            }

            _ => parse_call(working_set, &spans[pos..], spans[0]),
        }
    };

    if !shorthand.is_empty() {
        let with_env = working_set.find_decl(b"with-env");
        if let Some(decl_id) = with_env {
            let mut block = Block::default();
            let ty = output.ty.clone();
            block.pipelines = vec![Pipeline::from_vec(vec![output])];

            let block_id = working_set.add_block(Arc::new(block));

            let mut env_vars = vec![];
            for sh in shorthand {
                env_vars.push(RecordItem::Pair(sh.0, sh.1));
            }

            let arguments = vec![
                Argument::Positional(Expression {
                    expr: Expr::Record(env_vars),
                    span: Span::concat(&spans[..pos]),
                    ty: Type::Any,
                    custom_completion: None,
                }),
                Argument::Positional(Expression {
                    expr: Expr::Closure(block_id),
                    span: Span::concat(&spans[pos..]),
                    ty: Type::Closure,
                    custom_completion: None,
                }),
            ];

            let expr = Expr::Call(Box::new(Call {
                head: Span::unknown(),
                decl_id,
                arguments,
                parser_info: HashMap::new(),
            }));

            Expression {
                expr,
                custom_completion: None,
                span: Span::concat(spans),
                ty,
            }
        } else {
            output
        }
    } else {
        output
    }
}

pub fn parse_variable(working_set: &mut StateWorkingSet, span: Span) -> Option<VarId> {
    let bytes = working_set.get_span_contents(span);

    if is_variable(bytes) {
        working_set.find_variable(bytes)
    } else {
        working_set.error(ParseError::Expected("valid variable name", span));

        None
    }
}

pub fn parse_builtin_commands(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
) -> Pipeline {
    trace!("parsing: builtin commands");
    if !is_math_expression_like(working_set, lite_command.parts[0])
        && !is_unaliasable_parser_keyword(working_set, &lite_command.parts)
    {
        trace!("parsing: not math expression or unaliasable parser keyword");
        let name = working_set.get_span_contents(lite_command.parts[0]);
        if let Some(decl_id) = working_set.find_decl(name) {
            let cmd = working_set.get_decl(decl_id);
            if cmd.is_alias() {
                // Parse keywords that can be aliased. Note that we check for "unaliasable" keywords
                // because alias can have any name, therefore, we can't check for "aliasable" keywords.
                let call_expr = parse_call(working_set, &lite_command.parts, lite_command.parts[0]);

                if let Expression {
                    expr: Expr::Call(call),
                    ..
                } = call_expr
                {
                    // Apply parse keyword side effects
                    let cmd = working_set.get_decl(call.decl_id);
                    match cmd.name() {
                        "overlay hide" => return parse_overlay_hide(working_set, call),
                        "overlay new" => return parse_overlay_new(working_set, call),
                        "overlay use" => return parse_overlay_use(working_set, call),
                        _ => { /* this alias is not a parser keyword */ }
                    }
                }
            }
        }
    }

    trace!("parsing: checking for keywords");
    let name = working_set.get_span_contents(lite_command.parts[0]);

    match name {
        b"def" => parse_def(working_set, lite_command, None).0,
        b"extern" => parse_extern(working_set, lite_command, None),
        b"let" => parse_let(working_set, &lite_command.parts),
        b"const" => parse_const(working_set, &lite_command.parts),
        b"mut" => parse_mut(working_set, &lite_command.parts),
        b"for" => {
            let expr = parse_for(working_set, lite_command);
            Pipeline::from_vec(vec![expr])
        }
        b"alias" => parse_alias(working_set, lite_command, None),
        b"module" => parse_module(working_set, lite_command, None).0,
        b"use" => parse_use(working_set, lite_command).0,
        b"overlay" => {
            if let Some(redirection) = lite_command.redirection.as_ref() {
                working_set.error(redirecting_builtin_error("overlay", redirection));
                return garbage_pipeline(&lite_command.parts);
            }
            parse_keyword(working_set, lite_command)
        }
        b"source" | b"source-env" => parse_source(working_set, lite_command),
        b"export" => parse_export_in_block(working_set, lite_command),
        b"hide" => parse_hide(working_set, lite_command),
        b"where" => parse_where(working_set, lite_command),
        #[cfg(feature = "plugin")]
        b"register" => parse_register(working_set, lite_command),
        // Only "plugin use" is a keyword
        #[cfg(feature = "plugin")]
        b"plugin"
            if lite_command
                .parts
                .get(1)
                .is_some_and(|span| working_set.get_span_contents(*span) == b"use") =>
        {
            if let Some(redirection) = lite_command.redirection.as_ref() {
                working_set.error(redirecting_builtin_error("plugin use", redirection));
                return garbage_pipeline(&lite_command.parts);
            }
            parse_keyword(working_set, lite_command)
        }
        _ => {
            let element = parse_pipeline_element(working_set, lite_command);

            Pipeline {
                elements: vec![element],
            }
        }
    }
}

pub fn parse_record(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let bytes = working_set.get_span_contents(span);

    let mut start = span.start;
    let mut end = span.end;

    if bytes.starts_with(b"{") {
        start += 1;
    } else {
        working_set.error(ParseError::Expected("{", Span::new(start, start + 1)));
        return garbage(span);
    }

    if bytes.ends_with(b"}") {
        end -= 1;
    } else {
        working_set.error(ParseError::Unclosed("}".into(), Span::new(end, end)));
    }

    let inner_span = Span::new(start, end);
    let source = working_set.get_span_contents(inner_span);

    let (tokens, err) = lex(source, start, &[b'\n', b'\r', b','], &[b':'], true);
    if let Some(err) = err {
        working_set.error(err);
    }

    let mut output = vec![];
    let mut idx = 0;

    let mut field_types = Some(vec![]);
    while idx < tokens.len() {
        let curr_span = tokens[idx].span;
        let curr_tok = working_set.get_span_contents(curr_span);
        if curr_tok.starts_with(b"...")
            && curr_tok.len() > 3
            && (curr_tok[3] == b'$' || curr_tok[3] == b'{' || curr_tok[3] == b'(')
        {
            // Parse spread operator
            let inner = parse_value(
                working_set,
                Span::new(curr_span.start + 3, curr_span.end),
                &SyntaxShape::Record(vec![]),
            );
            idx += 1;

            match &inner.ty {
                Type::Record(inner_fields) => {
                    if let Some(fields) = &mut field_types {
                        for (field, ty) in inner_fields.as_ref() {
                            fields.push((field.clone(), ty.clone()));
                        }
                    }
                }
                _ => {
                    // We can't properly see all the field types
                    // so fall back to the Any type later
                    field_types = None;
                }
            }
            output.push(RecordItem::Spread(
                Span::new(curr_span.start, curr_span.start + 3),
                inner,
            ));
        } else {
            // Normal key-value pair
            let field = parse_value(working_set, curr_span, &SyntaxShape::Any);

            idx += 1;
            if idx == tokens.len() {
                working_set.error(ParseError::Expected(
                    "':'",
                    Span::new(curr_span.end, curr_span.end),
                ));
                output.push(RecordItem::Pair(
                    garbage(curr_span),
                    garbage(Span::new(curr_span.end, curr_span.end)),
                ));
                break;
            }
            let colon_span = tokens[idx].span;
            let colon = working_set.get_span_contents(colon_span);
            idx += 1;
            if colon != b":" {
                working_set.error(ParseError::Expected(
                    "':'",
                    Span::new(colon_span.start, colon_span.start),
                ));
                output.push(RecordItem::Pair(
                    field,
                    garbage(Span::new(
                        colon_span.start,
                        tokens[tokens.len() - 1].span.end,
                    )),
                ));
                break;
            }
            if idx == tokens.len() {
                working_set.error(ParseError::Expected(
                    "value for record field",
                    Span::new(colon_span.end, colon_span.end),
                ));
                output.push(RecordItem::Pair(
                    garbage(Span::new(curr_span.start, colon_span.end)),
                    garbage(Span::new(colon_span.end, tokens[tokens.len() - 1].span.end)),
                ));
                break;
            }
            let value = parse_value(working_set, tokens[idx].span, &SyntaxShape::Any);
            idx += 1;

            if let Some(field) = field.as_string() {
                if let Some(fields) = &mut field_types {
                    fields.push((field, value.ty.clone()));
                }
            } else {
                // We can't properly see all the field types
                // so fall back to the Any type later
                field_types = None;
            }
            output.push(RecordItem::Pair(field, value));
        }
    }

    Expression {
        expr: Expr::Record(output),
        span,
        ty: (if let Some(fields) = field_types {
            Type::Record(fields.into())
        } else {
            Type::Any
        }),
        custom_completion: None,
    }
}

fn parse_redirection_target(
    working_set: &mut StateWorkingSet,
    target: &LiteRedirectionTarget,
) -> RedirectionTarget {
    match target {
        LiteRedirectionTarget::File {
            connector,
            file,
            append,
        } => RedirectionTarget::File {
            expr: parse_value(working_set, *file, &SyntaxShape::Any),
            append: *append,
            span: *connector,
        },
        LiteRedirectionTarget::Pipe { connector } => RedirectionTarget::Pipe { span: *connector },
    }
}

pub(crate) fn parse_redirection(
    working_set: &mut StateWorkingSet,
    target: &LiteRedirection,
) -> PipelineRedirection {
    match target {
        LiteRedirection::Single { source, target } => PipelineRedirection::Single {
            source: *source,
            target: parse_redirection_target(working_set, target),
        },
        LiteRedirection::Separate { out, err } => PipelineRedirection::Separate {
            out: parse_redirection_target(working_set, out),
            err: parse_redirection_target(working_set, err),
        },
    }
}

fn parse_pipeline_element(
    working_set: &mut StateWorkingSet,
    command: &LiteCommand,
) -> PipelineElement {
    trace!("parsing: pipeline element");

    let expr = parse_expression(working_set, &command.parts);

    let redirection = command
        .redirection
        .as_ref()
        .map(|r| parse_redirection(working_set, r));

    PipelineElement {
        pipe: command.pipe,
        expr,
        redirection,
    }
}

pub(crate) fn redirecting_builtin_error(
    name: &'static str,
    redirection: &LiteRedirection,
) -> ParseError {
    match redirection {
        LiteRedirection::Single { target, .. } => {
            ParseError::RedirectingBuiltinCommand(name, target.connector(), None)
        }
        LiteRedirection::Separate { out, err } => ParseError::RedirectingBuiltinCommand(
            name,
            out.connector().min(err.connector()),
            Some(out.connector().max(err.connector())),
        ),
    }
}

pub fn parse_pipeline(
    working_set: &mut StateWorkingSet,
    pipeline: &LitePipeline,
    is_subexpression: bool,
    pipeline_index: usize,
) -> Pipeline {
    if pipeline.commands.len() > 1 {
        // Special case: allow `let` and `mut` to consume the whole pipeline, eg) `let abc = "foo" | str length`
        if let Some(&first) = pipeline.commands[0].parts.first() {
            let first = working_set.get_span_contents(first);
            if first == b"let" || first == b"mut" {
                let name = if first == b"let" { "let" } else { "mut" };
                let mut new_command = LiteCommand {
                    comments: vec![],
                    parts: pipeline.commands[0].parts.clone(),
                    pipe: None,
                    redirection: None,
                };

                if let Some(redirection) = pipeline.commands[0].redirection.as_ref() {
                    working_set.error(redirecting_builtin_error(name, redirection));
                }

                for element in &pipeline.commands[1..] {
                    if let Some(redirection) = pipeline.commands[0].redirection.as_ref() {
                        working_set.error(redirecting_builtin_error(name, redirection));
                    } else {
                        new_command.parts.push(element.pipe.expect("pipe span"));
                        new_command.comments.extend_from_slice(&element.comments);
                        new_command.parts.extend_from_slice(&element.parts);
                    }
                }

                // if the 'let' is complete enough, use it, if not, fall through for now
                if new_command.parts.len() > 3 {
                    let rhs_span = Span::concat(&new_command.parts[3..]);

                    new_command.parts.truncate(3);
                    new_command.parts.push(rhs_span);

                    let mut pipeline = parse_builtin_commands(working_set, &new_command);

                    if pipeline_index == 0 {
                        let let_decl_id = working_set.find_decl(b"let");
                        let mut_decl_id = working_set.find_decl(b"mut");
                        for element in pipeline.elements.iter_mut() {
                            if let Expr::Call(call) = &element.expr.expr {
                                if Some(call.decl_id) == let_decl_id
                                    || Some(call.decl_id) == mut_decl_id
                                {
                                    // Do an expansion
                                    if let Some(Expression {
                                        expr: Expr::Block(block_id),
                                        ..
                                    }) = call.positional_iter().nth(1)
                                    {
                                        let block = working_set.get_block(*block_id);

                                        if let Some(element) = block
                                            .pipelines
                                            .first()
                                            .and_then(|p| p.elements.first())
                                            .cloned()
                                        {
                                            if element.has_in_variable(working_set) {
                                                let element = wrap_element_with_collect(
                                                    working_set,
                                                    &element,
                                                );
                                                let block = working_set.get_block_mut(*block_id);
                                                block.pipelines[0].elements[0] = element;
                                            }
                                        }
                                    }
                                    continue;
                                } else if element.has_in_variable(working_set) && !is_subexpression
                                {
                                    *element = wrap_element_with_collect(working_set, element);
                                }
                            } else if element.has_in_variable(working_set) && !is_subexpression {
                                *element = wrap_element_with_collect(working_set, element);
                            }
                        }
                    }

                    return pipeline;
                }
            }
        }

        let mut elements = pipeline
            .commands
            .iter()
            .map(|element| parse_pipeline_element(working_set, element))
            .collect::<Vec<_>>();

        if is_subexpression {
            for element in elements.iter_mut().skip(1) {
                if element.has_in_variable(working_set) {
                    *element = wrap_element_with_collect(working_set, element);
                }
            }
        } else {
            for element in elements.iter_mut() {
                if element.has_in_variable(working_set) {
                    *element = wrap_element_with_collect(working_set, element);
                }
            }
        }

        Pipeline { elements }
    } else {
        if let Some(&first) = pipeline.commands[0].parts.first() {
            let first = working_set.get_span_contents(first);
            if first == b"let" || first == b"mut" {
                if let Some(redirection) = pipeline.commands[0].redirection.as_ref() {
                    let name = if first == b"let" { "let" } else { "mut" };
                    working_set.error(redirecting_builtin_error(name, redirection));
                }
            }
        }

        let mut pipeline = parse_builtin_commands(working_set, &pipeline.commands[0]);

        let let_decl_id = working_set.find_decl(b"let");
        let mut_decl_id = working_set.find_decl(b"mut");

        if pipeline_index == 0 {
            for element in pipeline.elements.iter_mut() {
                if let Expr::Call(call) = &element.expr.expr {
                    if Some(call.decl_id) == let_decl_id || Some(call.decl_id) == mut_decl_id {
                        // Do an expansion
                        if let Some(Expression {
                            expr: Expr::Block(block_id),
                            ..
                        }) = call.positional_iter().nth(1)
                        {
                            let block = working_set.get_block(*block_id);

                            if let Some(element) = block
                                .pipelines
                                .first()
                                .and_then(|p| p.elements.first())
                                .cloned()
                            {
                                if element.has_in_variable(working_set) {
                                    let element = wrap_element_with_collect(working_set, &element);
                                    let block = working_set.get_block_mut(*block_id);
                                    block.pipelines[0].elements[0] = element;
                                }
                            }
                        }
                        continue;
                    } else if element.has_in_variable(working_set) && !is_subexpression {
                        *element = wrap_element_with_collect(working_set, element);
                    }
                } else if element.has_in_variable(working_set) && !is_subexpression {
                    *element = wrap_element_with_collect(working_set, element);
                }
            }
        }

        pipeline
    }
}

pub fn parse_block(
    working_set: &mut StateWorkingSet,
    tokens: &[Token],
    span: Span,
    scoped: bool,
    is_subexpression: bool,
) -> Block {
    let (lite_block, err) = lite_parse(tokens);
    if let Some(err) = err {
        working_set.error(err);
    }

    trace!("parsing block: {:?}", lite_block);

    if scoped {
        working_set.enter_scope();
    }

    // Pre-declare any definition so that definitions
    // that share the same block can see each other
    for pipeline in &lite_block.block {
        if pipeline.commands.len() == 1 {
            parse_def_predecl(working_set, &pipeline.commands[0].parts)
        }
    }

    let mut block = Block::new_with_capacity(lite_block.block.len());

    for (idx, lite_pipeline) in lite_block.block.iter().enumerate() {
        let pipeline = parse_pipeline(working_set, lite_pipeline, is_subexpression, idx);
        block.pipelines.push(pipeline);
    }

    if scoped {
        working_set.exit_scope();
    }

    block.span = Some(span);

    let errors = type_check::check_block_input_output(working_set, &block);
    if !errors.is_empty() {
        working_set.parse_errors.extend_from_slice(&errors);
    }

    block
}

pub fn discover_captures_in_closure(
    working_set: &StateWorkingSet,
    block: &Block,
    seen: &mut Vec<VarId>,
    seen_blocks: &mut HashMap<BlockId, Vec<(VarId, Span)>>,
    output: &mut Vec<(VarId, Span)>,
) -> Result<(), ParseError> {
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
        discover_captures_in_pipeline(working_set, pipeline, seen, seen_blocks, output)?;
    }

    Ok(())
}

fn discover_captures_in_pipeline(
    working_set: &StateWorkingSet,
    pipeline: &Pipeline,
    seen: &mut Vec<VarId>,
    seen_blocks: &mut HashMap<BlockId, Vec<(VarId, Span)>>,
    output: &mut Vec<(VarId, Span)>,
) -> Result<(), ParseError> {
    for element in &pipeline.elements {
        discover_captures_in_pipeline_element(working_set, element, seen, seen_blocks, output)?;
    }

    Ok(())
}

// Closes over captured variables
pub fn discover_captures_in_pipeline_element(
    working_set: &StateWorkingSet,
    element: &PipelineElement,
    seen: &mut Vec<VarId>,
    seen_blocks: &mut HashMap<BlockId, Vec<(VarId, Span)>>,
    output: &mut Vec<(VarId, Span)>,
) -> Result<(), ParseError> {
    discover_captures_in_expr(working_set, &element.expr, seen, seen_blocks, output)?;

    if let Some(redirection) = element.redirection.as_ref() {
        match redirection {
            PipelineRedirection::Single { target, .. } => {
                if let Some(expr) = target.expr() {
                    discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
                }
            }
            PipelineRedirection::Separate { out, err } => {
                if let Some(expr) = out.expr() {
                    discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
                }
                if let Some(expr) = err.expr() {
                    discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
                }
            }
        }
    }

    Ok(())
}

pub fn discover_captures_in_pattern(pattern: &MatchPattern, seen: &mut Vec<VarId>) {
    match &pattern.pattern {
        Pattern::Variable(var_id) => seen.push(*var_id),
        Pattern::List(items) => {
            for item in items {
                discover_captures_in_pattern(item, seen)
            }
        }
        Pattern::Record(items) => {
            for item in items {
                discover_captures_in_pattern(&item.1, seen)
            }
        }
        Pattern::Or(patterns) => {
            for pattern in patterns {
                discover_captures_in_pattern(pattern, seen)
            }
        }
        Pattern::Rest(var_id) => seen.push(*var_id),
        Pattern::Value(_) | Pattern::IgnoreValue | Pattern::IgnoreRest | Pattern::Garbage => {}
    }
}

// Closes over captured variables
pub fn discover_captures_in_expr(
    working_set: &StateWorkingSet,
    expr: &Expression,
    seen: &mut Vec<VarId>,
    seen_blocks: &mut HashMap<BlockId, Vec<(VarId, Span)>>,
    output: &mut Vec<(VarId, Span)>,
) -> Result<(), ParseError> {
    match &expr.expr {
        Expr::BinaryOp(lhs, _, rhs) => {
            discover_captures_in_expr(working_set, lhs, seen, seen_blocks, output)?;
            discover_captures_in_expr(working_set, rhs, seen, seen_blocks, output)?;
        }
        Expr::UnaryNot(expr) => {
            discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
        }
        Expr::Closure(block_id) => {
            let block = working_set.get_block(*block_id);
            let results = {
                let mut seen = vec![];
                let mut results = vec![];

                discover_captures_in_closure(
                    working_set,
                    block,
                    &mut seen,
                    seen_blocks,
                    &mut results,
                )?;

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
                let mut results = vec![];
                discover_captures_in_closure(
                    working_set,
                    block,
                    &mut seen,
                    seen_blocks,
                    &mut results,
                )?;
                results
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
            if let Some(block_id) = decl.block_id() {
                match seen_blocks.get(&block_id) {
                    Some(capture_list) => {
                        // Push captures onto the outer closure that aren't created by that outer closure
                        for capture in capture_list {
                            if !seen.contains(&capture.0) {
                                output.push(*capture);
                            }
                        }
                    }
                    None => {
                        let block = working_set.get_block(block_id);
                        if !block.captures.is_empty() {
                            for capture in &block.captures {
                                if !seen.contains(capture) {
                                    output.push((*capture, call.head));
                                }
                            }
                        } else {
                            let result = {
                                let mut seen = vec![];
                                seen_blocks.insert(block_id, output.clone());

                                let mut result = vec![];
                                discover_captures_in_closure(
                                    working_set,
                                    block,
                                    &mut seen,
                                    seen_blocks,
                                    &mut result,
                                )?;

                                result
                            };
                            // Push captures onto the outer closure that aren't created by that outer closure
                            for capture in &result {
                                if !seen.contains(&capture.0) {
                                    output.push(*capture);
                                }
                            }

                            seen_blocks.insert(block_id, result);
                        }
                    }
                }
            }

            for arg in &call.arguments {
                match arg {
                    Argument::Named(named) => {
                        if let Some(arg) = &named.2 {
                            discover_captures_in_expr(working_set, arg, seen, seen_blocks, output)?;
                        }
                    }
                    Argument::Positional(expr)
                    | Argument::Unknown(expr)
                    | Argument::Spread(expr) => {
                        discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
                    }
                }
            }
        }
        Expr::CellPath(_) => {}
        Expr::DateTime(_) => {}
        Expr::ExternalCall(head, args) => {
            discover_captures_in_expr(working_set, head, seen, seen_blocks, output)?;

            for ExternalArgument::Regular(expr) | ExternalArgument::Spread(expr) in args.as_ref() {
                discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
            }
        }
        Expr::Filepath(_, _) => {}
        Expr::Directory(_, _) => {}
        Expr::Float(_) => {}
        Expr::FullCellPath(cell_path) => {
            discover_captures_in_expr(working_set, &cell_path.head, seen, seen_blocks, output)?;
        }
        Expr::ImportPattern(_) => {}
        Expr::Overlay(_) => {}
        Expr::Garbage => {}
        Expr::Nothing => {}
        Expr::GlobPattern(_, _) => {}
        Expr::Int(_) => {}
        Expr::Keyword(kw) => {
            discover_captures_in_expr(working_set, &kw.expr, seen, seen_blocks, output)?;
        }
        Expr::List(list) => {
            for item in list {
                discover_captures_in_expr(working_set, item.expr(), seen, seen_blocks, output)?;
            }
        }
        Expr::Operator(_) => {}
        Expr::Range(range) => {
            if let Some(from) = &range.from {
                discover_captures_in_expr(working_set, from, seen, seen_blocks, output)?;
            }
            if let Some(next) = &range.next {
                discover_captures_in_expr(working_set, next, seen, seen_blocks, output)?;
            }
            if let Some(to) = &range.to {
                discover_captures_in_expr(working_set, to, seen, seen_blocks, output)?;
            }
        }
        Expr::Record(items) => {
            for item in items {
                match item {
                    RecordItem::Pair(field_name, field_value) => {
                        discover_captures_in_expr(
                            working_set,
                            field_name,
                            seen,
                            seen_blocks,
                            output,
                        )?;
                        discover_captures_in_expr(
                            working_set,
                            field_value,
                            seen,
                            seen_blocks,
                            output,
                        )?;
                    }
                    RecordItem::Spread(_, record) => {
                        discover_captures_in_expr(working_set, record, seen, seen_blocks, output)?;
                    }
                }
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
        Expr::RawString(_) => {}
        Expr::StringInterpolation(exprs) => {
            for expr in exprs {
                discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
            }
        }
        Expr::MatchBlock(match_block) => {
            for match_ in match_block {
                discover_captures_in_pattern(&match_.0, seen);
                discover_captures_in_expr(working_set, &match_.1, seen, seen_blocks, output)?;
            }
        }
        Expr::RowCondition(block_id) | Expr::Subexpression(block_id) => {
            let block = working_set.get_block(*block_id);

            let results = {
                let mut results = vec![];
                let mut seen = vec![];
                discover_captures_in_closure(
                    working_set,
                    block,
                    &mut seen,
                    seen_blocks,
                    &mut results,
                )?;
                results
            };

            seen_blocks.insert(*block_id, results.clone());
            for (var_id, span) in results.into_iter() {
                if !seen.contains(&var_id) {
                    output.push((var_id, span))
                }
            }
        }
        Expr::Table(table) => {
            for header in table.columns.as_ref() {
                discover_captures_in_expr(working_set, header, seen, seen_blocks, output)?;
            }
            for row in table.rows.as_ref() {
                for cell in row.as_ref() {
                    discover_captures_in_expr(working_set, cell, seen, seen_blocks, output)?;
                }
            }
        }
        Expr::ValueWithUnit(value) => {
            discover_captures_in_expr(working_set, &value.expr, seen, seen_blocks, output)?;
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
    Ok(())
}

fn wrap_redirection_with_collect(
    working_set: &mut StateWorkingSet,
    target: &RedirectionTarget,
) -> RedirectionTarget {
    match target {
        RedirectionTarget::File { expr, append, span } => RedirectionTarget::File {
            expr: wrap_expr_with_collect(working_set, expr),
            span: *span,
            append: *append,
        },
        RedirectionTarget::Pipe { span } => RedirectionTarget::Pipe { span: *span },
    }
}

fn wrap_element_with_collect(
    working_set: &mut StateWorkingSet,
    element: &PipelineElement,
) -> PipelineElement {
    PipelineElement {
        pipe: element.pipe,
        expr: wrap_expr_with_collect(working_set, &element.expr),
        redirection: element.redirection.as_ref().map(|r| match r {
            PipelineRedirection::Single { source, target } => PipelineRedirection::Single {
                source: *source,
                target: wrap_redirection_with_collect(working_set, target),
            },
            PipelineRedirection::Separate { out, err } => PipelineRedirection::Separate {
                out: wrap_redirection_with_collect(working_set, out),
                err: wrap_redirection_with_collect(working_set, err),
            },
        }),
    }
}

fn wrap_expr_with_collect(working_set: &mut StateWorkingSet, expr: &Expression) -> Expression {
    let span = expr.span;

    if let Some(decl_id) = working_set.find_decl(b"collect") {
        let mut output = vec![];

        let var_id = IN_VARIABLE_ID;
        let mut signature = Signature::new("");
        signature.required_positional.push(PositionalArg {
            var_id: Some(var_id),
            name: "$in".into(),
            desc: String::new(),
            shape: SyntaxShape::Any,
            default_value: None,
        });

        let block = Block {
            pipelines: vec![Pipeline::from_vec(vec![expr.clone()])],
            signature: Box::new(signature),
            ..Default::default()
        };

        let block_id = working_set.add_block(Arc::new(block));

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
                parser_info: HashMap::new(),
            })),
            span,
            ty: Type::Any,
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
) -> Arc<Block> {
    trace!("parse");
    let name = match fname {
        Some(fname) => {
            // use the canonical name for this filename
            nu_path::expand_to_real_path(fname)
                .to_string_lossy()
                .to_string()
        }
        None => "source".to_string(),
    };

    let file_id = working_set.add_file(name, contents);
    let new_span = working_set.get_span_for_file(file_id);

    let previously_parsed_block = working_set.find_block_by_span(new_span);

    let mut output = {
        if let Some(block) = previously_parsed_block {
            // dbg!("previous block");
            return block;
        } else {
            // dbg!("starting lex");
            let (output, err) = lex(contents, new_span.start, &[], &[], false);
            // dbg!("finished lex");
            // dbg!(&output);
            if let Some(err) = err {
                working_set.error(err)
            }

            Arc::new(parse_block(working_set, &output, new_span, scoped, false))
        }
    };

    let mut seen = vec![];
    let mut seen_blocks = HashMap::new();

    let mut captures = vec![];
    match discover_captures_in_closure(
        working_set,
        &output,
        &mut seen,
        &mut seen_blocks,
        &mut captures,
    ) {
        Ok(_) => {
            Arc::make_mut(&mut output).captures =
                captures.into_iter().map(|(var_id, _)| var_id).collect();
        }
        Err(err) => working_set.error(err),
    }

    // Also check other blocks that might have been imported
    let mut errors = vec![];
    for (block_idx, block) in working_set.delta.blocks.iter().enumerate() {
        let block_id = block_idx + working_set.permanent_state.num_blocks();

        if !seen_blocks.contains_key(&block_id) {
            let mut captures = vec![];

            match discover_captures_in_closure(
                working_set,
                block,
                &mut seen,
                &mut seen_blocks,
                &mut captures,
            ) {
                Ok(_) => {
                    seen_blocks.insert(block_id, captures);
                }
                Err(err) => {
                    errors.push(err);
                }
            }
        }
    }
    for err in errors {
        working_set.error(err)
    }

    for (block_id, captures) in seen_blocks.into_iter() {
        // In theory, we should only be updating captures where we have new information
        // the only place where this is possible would be blocks that are newly created
        // by our working set delta. If we ever tried to modify the permanent state, we'd
        // panic (again, in theory, this shouldn't be possible)
        let block = working_set.get_block(block_id);
        let block_captures_empty = block.captures.is_empty();
        // need to check block_id >= working_set.permanent_state.num_blocks()
        // to avoid mutate a block that is in the permanent state.
        // this can happened if user defines a function with recursive call
        // and pipe a variable to the command, e.g:
        // def px [] { if true { 42 } else { px } };    # the block px is saved in permanent state.
        // let x = 3
        // $x | px
        // If we don't guard for `block_id`, it will change captures of `px`, which is
        // already saved in permanent state
        if !captures.is_empty()
            && block_captures_empty
            && block_id >= working_set.permanent_state.num_blocks()
        {
            let block = working_set.get_block_mut(block_id);
            block.captures = captures.into_iter().map(|(var_id, _)| var_id).collect();
        }
    }

    output
}
