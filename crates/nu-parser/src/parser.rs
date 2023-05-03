use crate::{
    eval::{eval_constant, value_as_string},
    lex::{lex, lex_signature},
    lite_parser::{lite_parse, LiteCommand, LiteElement},
    parse_mut,
    parse_patterns::{parse_match_pattern, parse_pattern},
    type_check::{math_result_type, type_compatible},
    Token, TokenContents,
};

use nu_engine::DIR_VAR_PARSER_INFO;
use nu_protocol::{
    ast::{
        Argument, Assignment, Bits, Block, Boolean, Call, CellPath, Comparison, Expr, Expression,
        FullCellPath, ImportPattern, ImportPatternHead, ImportPatternMember, MatchPattern, Math,
        Operator, PathMember, Pattern, Pipeline, PipelineElement, RangeInclusion, RangeOperator,
    },
    engine::StateWorkingSet,
    span, BlockId, DidYouMean, Flag, ParseError, PositionalArg, Signature, Span, Spanned,
    SyntaxShape, Type, Unit, VarId, ENV_VARIABLE_ID, IN_VARIABLE_ID,
};

use crate::parse_keywords::{
    find_dirs_var, is_unaliasable_parser_keyword, parse_alias, parse_def, parse_def_predecl,
    parse_export_in_block, parse_extern, parse_for, parse_hide, parse_keyword, parse_let_or_const,
    parse_module, parse_overlay_hide, parse_overlay_new, parse_overlay_use, parse_source,
    parse_use, parse_where, parse_where_expr, LIB_DIRS_VAR,
};

use itertools::Itertools;
use log::trace;
use std::{
    collections::{HashMap, HashSet},
    num::ParseIntError,
    str,
};

#[cfg(feature = "plugin")]
use crate::parse_keywords::parse_register;

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

pub fn check_call(working_set: &mut StateWorkingSet, command: Span, sig: &Signature, call: &Call) {
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

pub fn check_name<'a>(working_set: &mut StateWorkingSet, spans: &'a [Span]) -> Option<&'a Span> {
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
            working_set.error(ParseError::AssignmentMismatch(
                format!("{name} missing name"),
                "missing name".into(),
                spans[command_len],
            ));
            Some(&spans[command_len])
        } else {
            None
        }
    } else if working_set.get_span_contents(spans[command_len + 1]) != b"=" {
        let name =
            String::from_utf8_lossy(working_set.get_span_contents(span(&spans[..command_len])));
        working_set.error(ParseError::AssignmentMismatch(
            format!("{name} missing sign"),
            "missing equal sign".into(),
            spans[command_len + 1],
        ));
        Some(&spans[command_len + 1])
    } else {
        None
    }
}

fn parse_external_arg(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let contents = working_set.get_span_contents(span);

    if contents.starts_with(b"$") || contents.starts_with(b"(") {
        parse_dollar_expr(working_set, span)
    } else if contents.starts_with(b"[") {
        parse_list_expression(working_set, span, &SyntaxShape::Any)
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

        Expression {
            expr: Expr::String(contents),
            span,
            ty: Type::String,
            custom_completion: None,
        }
    }
}

pub fn parse_external_call(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    is_subexpression: bool,
) -> Expression {
    trace!("parse external");

    let mut args = vec![];

    let head_contents = working_set.get_span_contents(spans[0]);

    let head_span = if head_contents.starts_with(b"^") {
        Span::new(spans[0].start + 1, spans[0].end)
    } else {
        spans[0]
    };

    let head_contents = working_set.get_span_contents(head_span).to_vec();

    let head = if head_contents.starts_with(b"$") || head_contents.starts_with(b"(") {
        // the expression is inside external_call, so it's a subexpression
        let arg = parse_expression(working_set, &[head_span], true);
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

    for span in &spans[1..] {
        let arg = parse_external_arg(working_set, *span);
        args.push(arg);
    }

    Expression {
        expr: Expr::ExternalCall(head, args, is_subexpression),
        span: span(spans),
        ty: Type::Any,
        custom_completion: None,
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

                        (
                            Some(Spanned {
                                item: long_name,
                                span: Span::new(arg_span.start, arg_span.start + long_name_len + 2),
                            }),
                            Some(arg),
                        )
                    } else if let Some(arg) = spans.get(*spans_idx + 1) {
                        let arg = parse_value(working_set, *arg, arg_shape);

                        *spans_idx += 1;
                        (
                            Some(Spanned {
                                item: long_name,
                                span: arg_span,
                            }),
                            Some(arg),
                        )
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
                    (
                        Some(Spanned {
                            item: long_name,
                            span: arg_span,
                        }),
                        None,
                    )
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

            if found_short_flags.is_empty() {
                let arg_contents = working_set.get_span_contents(arg_span);

                // check to see if we have a negative number
                if let Some(positional) = sig.get_positional(positional_idx) {
                    if positional.shape == SyntaxShape::Int
                        || positional.shape == SyntaxShape::Number
                    {
                        if String::from_utf8_lossy(arg_contents).parse::<f64>().is_ok() {
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
                    } else if let Some(first) = unmatched_short_flags.first() {
                        let contents = working_set.get_span_contents(*first);
                        working_set.error(ParseError::UnknownFlag(
                            sig.name.clone(),
                            format!("-{}", String::from_utf8_lossy(contents)),
                            *first,
                            sig.clone().formatted_flags(),
                        ));
                    }
                } else if let Some(first) = unmatched_short_flags.first() {
                    let contents = working_set.get_span_contents(*first);
                    working_set.error(ParseError::UnknownFlag(
                        sig.name.clone(),
                        format!("-{}", String::from_utf8_lossy(contents)),
                        *first,
                        sig.clone().formatted_flags(),
                    ));
                }
            } else if !unmatched_short_flags.is_empty() {
                if let Some(first) = unmatched_short_flags.first() {
                    let contents = working_set.get_span_contents(*first);
                    working_set.error(ParseError::UnknownFlag(
                        sig.name.clone(),
                        format!("-{}", String::from_utf8_lossy(contents)),
                        *first,
                        sig.clone().formatted_flags(),
                    ));
                }
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
) -> Expression {
    match shape {
        SyntaxShape::VarWithOptType => {
            trace!("parsing: var with opt type");

            parse_var_with_opt_type(working_set, spans, spans_idx, false)
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
                } else if let Some(ParseError::Expected(..)) = working_set.parse_errors.last() {
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
                working_set.error(ParseError::Expected(
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
            let arg = parse_expression(working_set, &spans[*spans_idx..], false);
            *spans_idx = spans.len() - 1;

            arg
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
                return Expression {
                    expr: Expr::Keyword(
                        keyword.clone(),
                        spans[*spans_idx - 1],
                        Box::new(Expression::garbage(arg_span)),
                    ),
                    span: arg_span,
                    ty: Type::Any,
                    custom_completion: None,
                };
            }
            let keyword_span = spans[*spans_idx - 1];
            let expr = parse_multispan_value(working_set, spans, spans_idx, arg);
            let ty = expr.ty.clone();

            Expression {
                expr: Expr::Keyword(keyword.clone(), keyword_span, Box::new(expr)),
                span: arg_span,
                ty,
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
    let output = signature.output_type.clone();

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

    working_set.type_scope.add_type(output.clone());

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

pub fn parse_call(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    head: Span,
    is_subexpression: bool,
) -> Expression {
    trace!("parsing: call");

    if spans.is_empty() {
        working_set.error(ParseError::UnknownState(
            "Encountered command with zero spans".into(),
            span(spans),
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

                working_set.error(ParseError::UnknownState(
                    "Incomplete statement".into(),
                    span(spans),
                ));
                return garbage(span(spans));
            }
        }

        // TODO: Try to remove the clone
        let decl = working_set.get_decl(decl_id).clone();

        let parsed_call = if let Some(alias) = decl.as_alias() {
            if let Expression {
                expr: Expr::ExternalCall(head, args, is_subexpression),
                span: _,
                ty,
                custom_completion,
            } = &alias.wrapped_call
            {
                trace!("parsing: alias of external call");

                let mut final_args = args.clone();

                for arg_span in spans.iter().skip(1) {
                    let arg = parse_external_arg(working_set, *arg_span);
                    final_args.push(arg);
                }

                let mut head = head.clone();
                head.span = spans[0]; // replacing the spans preserves syntax highlighting

                return Expression {
                    expr: Expr::ExternalCall(head, final_args, *is_subexpression),
                    span: span(spans),
                    ty: ty.clone(),
                    custom_completion: *custom_completion,
                };
            } else {
                trace!("parsing: alias of internal call");
                parse_internal_call(
                    working_set,
                    span(&spans[cmd_start..pos]),
                    &spans[pos..],
                    decl_id,
                )
            }
        } else {
            trace!("parsing: internal call");
            parse_internal_call(
                working_set,
                span(&spans[cmd_start..pos]),
                &spans[pos..],
                decl_id,
            )
        };

        Expression {
            expr: Expr::Call(parsed_call.call),
            span: span(spans),
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
        parse_external_call(working_set, spans, is_subexpression)
    }
}

pub fn parse_binary(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let contents = working_set.get_span_contents(span);
    if contents.starts_with(b"0x[") {
        parse_binary_with_base(working_set, span, 16, 2, b"0x[", b"]")
    } else if contents.starts_with(b"0o[") {
        parse_binary_with_base(working_set, span, 8, 3, b"0o[", b"]")
    } else {
        parse_binary_with_base(working_set, span, 2, 8, b"0b[", b"]")
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
                    | TokenContents::OutGreaterThan
                    | TokenContents::ErrGreaterThan
                    | TokenContents::OutErrGreaterThan => {
                        working_set.error(ParseError::Expected("binary".into(), span));
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

    working_set.error(ParseError::Expected("binary".into(), span));
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
        working_set.error(ParseError::Expected("int".into(), span));
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
        working_set.error(ParseError::Expected("int".into(), span));
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
        working_set.error(ParseError::Expected("float".into(), span));

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

    working_set.error(ParseError::Expected("number".into(), span));
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
        working_set.error(ParseError::Expected(
            "at least one range bound set".into(),
            span,
        ));
        return garbage(span);
    }

    // First, figure out what exact operators are used and determine their positions
    let dotdot_pos: Vec<_> = token.match_indices("..").map(|(pos, _)| pos).collect();

    let (next_op_pos, range_op_pos) = match dotdot_pos.len() {
        1 => (None, dotdot_pos[0]),
        2 => (Some(dotdot_pos[0]), dotdot_pos[1]),
        _ => {
            working_set.error(ParseError::Expected(
                "one range operator ('..' or '..<') and optionally one next operator ('..')".into(),
                span,
            ));
            return garbage(span);
        }
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
            working_set.error(ParseError::Expected(
                "inclusive operator preceding second range bound".into(),
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
        Some(Box::new(parse_value(
            working_set,
            from_span,
            &SyntaxShape::Number,
        )))
    };

    let to = if token.ends_with(range_op_str) {
        None
    } else {
        let to_span = Span::new(range_op_span.end, span.end);
        Some(Box::new(parse_value(
            working_set,
            to_span,
            &SyntaxShape::Number,
        )))
    };

    trace!("-- from: {:?} to: {:?}", from, to);

    if let (None, None) = (&from, &to) {
        working_set.error(ParseError::Expected(
            "at least one range bound set".into(),
            span,
        ));
        return garbage(span);
    }

    let (next, next_op_span) = if let Some(pos) = next_op_pos {
        let next_op_span = Span::new(span.start + pos, span.start + pos + "..".len());
        let next_span = Span::new(next_op_span.end, range_op_span.start);

        (
            Some(Box::new(parse_value(
                working_set,
                next_span,
                &SyntaxShape::Number,
            ))),
            next_op_span,
        )
    } else {
        (None, span)
    };

    let range_op = RangeOperator {
        inclusion,
        span: range_op_span,
        next_op_span,
    };

    Expression {
        expr: Expr::Range(from, next, to, range_op),
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
        working_set.error(ParseError::Expected(
            format!("non-block value: {shape}"),
            span,
        ));
        return Expression::garbage(span);
    }

    let bytes = working_set.get_span_contents(Span::new(span.start + 1, span.end - 1));
    let (tokens, _) = lex(bytes, span.start + 1, &[b'\r', b'\n', b'\t'], &[b':'], true);

    let second_token = tokens
        .get(0)
        .map(|token| working_set.get_span_contents(token.span));

    let second_token_contents = tokens.get(0).map(|token| token.contents);

    let third_token = tokens
        .get(1)
        .map(|token| working_set.get_span_contents(token.span));

    if matches!(second_token, None) {
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
    } else if matches!(shape, SyntaxShape::Closure(_)) || matches!(shape, SyntaxShape::Any) {
        parse_closure_expression(working_set, shape, span)
    } else if matches!(shape, SyntaxShape::Block) {
        parse_block_expression(working_set, span)
    } else if matches!(shape, SyntaxShape::MatchBlock) {
        parse_match_block_expression(working_set, span)
    } else {
        working_set.error(ParseError::Expected(
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

    if contents == b"$nothing" {
        return Expression {
            expr: Expr::Nothing,
            span,
            ty: Type::Nothing,
            custom_completion: None,
        };
    } else if contents == b"$nu" {
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

    if let Some(id) = parse_variable(working_set, span) {
        Expression {
            expr: Expr::Var(id),
            span,
            ty: working_set.get_variable(id).ty.clone(),
            custom_completion: None,
        }
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
                    working_set.error(ParseError::Expected('.'.into(), path_element.span));
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
                    working_set.error(ParseError::Expected(". or ?".into(), path_element.span));
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
                                working_set.error(ParseError::Expected(
                                    "string".into(),
                                    path_element.span,
                                ));
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
            working_set
                .type_scope
                .add_type(working_set.type_scope.get_last_output());

            let ty = output
                .pipelines
                .last()
                .and_then(|Pipeline { elements, .. }| elements.last())
                .map(|element| match element {
                    PipelineElement::Expression(_, expr)
                        if matches!(
                            expr,
                            Expression {
                                expr: Expr::BinaryOp(..),
                                ..
                            }
                        ) =>
                    {
                        expr.ty.clone()
                    }
                    _ => working_set.type_scope.get_last_output(),
                })
                .unwrap_or_else(|| working_set.type_scope.get_last_output());

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
    let (token, err) = unescape_unquote_string(bytes, span);
    trace!("parsing: directory");

    if err.is_none() {
        trace!("-- found {}", token);

        Expression {
            expr: Expr::Directory(token),
            span,
            ty: Type::String,
            custom_completion: None,
        }
    } else {
        working_set.error(ParseError::Expected("directory".into(), span));

        garbage(span)
    }
}

pub fn parse_filepath(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let bytes = working_set.get_span_contents(span);
    let (token, err) = unescape_unquote_string(bytes, span);
    trace!("parsing: filepath");

    if err.is_none() {
        trace!("-- found {}", token);

        Expression {
            expr: Expr::Filepath(token),
            span,
            ty: Type::String,
            custom_completion: None,
        }
    } else {
        working_set.error(ParseError::Expected("filepath".into(), span));

        garbage(span)
    }
}
/// Parse a datetime type, eg '2022-02-02'
pub fn parse_datetime(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: datetime");

    let bytes = working_set.get_span_contents(span);

    if bytes.len() < 5
        || !bytes[0].is_ascii_digit()
        || !bytes[1].is_ascii_digit()
        || !bytes[2].is_ascii_digit()
        || !bytes[3].is_ascii_digit()
    {
        working_set.error(ParseError::Expected("datetime".into(), span));
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

    working_set.error(ParseError::Expected("datetime".into(), span));

    garbage(span)
}

/// Parse a duration type, eg '10day'
pub fn parse_duration(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: duration");

    let bytes = working_set.get_span_contents(span);

    match parse_duration_bytes(bytes, span) {
        Some(expression) => expression,
        None => {
            working_set.error(ParseError::Expected(
                "duration with valid units".into(),
                span,
            ));

            garbage(span)
        }
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
    let unit_groups = [
        (Unit::Nanosecond, "ns", None),
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

    if let Some(unit) = unit_groups.iter().find(|&x| num_with_unit.ends_with(x.1)) {
        let mut lhs = num_with_unit;
        for _ in 0..unit.1.chars().count() {
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
pub fn parse_filesize(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: filesize");

    let bytes = working_set.get_span_contents(span);

    //todo: parse_filesize_bytes should distinguish between not-that-type and syntax error in units
    match parse_filesize_bytes(bytes, span) {
        Some(expression) => expression,
        None => {
            working_set.error(ParseError::Expected(
                "filesize with valid units".into(),
                span,
            ));

            garbage(span)
        }
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

pub fn parse_glob_pattern(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let bytes = working_set.get_span_contents(span);
    let (token, err) = unescape_unquote_string(bytes, span);
    trace!("parsing: glob pattern");

    if err.is_none() {
        trace!("-- found {}", token);

        Expression {
            expr: Expr::GlobPattern(token),
            span,
            ty: Type::String,
            custom_completion: None,
        }
    } else {
        working_set.error(ParseError::Expected("glob pattern string".into(), span));

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

pub fn parse_string(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: string");

    let bytes = working_set.get_span_contents(span);

    if bytes.is_empty() {
        working_set.error(ParseError::Expected("String".into(), span));
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
            working_set.error(ParseError::Expected("string".into(), span));

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
        working_set.error(ParseError::Expected("string".into(), span));
        garbage(span)
    }
}

//TODO: Handle error case for unknown shapes
pub fn parse_shape_name(
    working_set: &mut StateWorkingSet,
    bytes: &[u8],
    span: Span,
) -> SyntaxShape {
    let result = match bytes {
        b"any" => SyntaxShape::Any,
        b"binary" => SyntaxShape::Binary,
        b"block" => SyntaxShape::Block, //FIXME: Blocks should have known output types
        b"bool" => SyntaxShape::Boolean,
        b"cell-path" => SyntaxShape::CellPath,
        b"closure" => SyntaxShape::Closure(None), //FIXME: Blocks should have known output types
        b"cond" => SyntaxShape::RowCondition,
        // b"custom" => SyntaxShape::Custom(Box::new(SyntaxShape::Any), SyntaxShape::Int),
        b"datetime" => SyntaxShape::DateTime,
        b"directory" => SyntaxShape::Directory,
        b"duration" => SyntaxShape::Duration,
        b"error" => SyntaxShape::Error,
        b"expr" => SyntaxShape::Expression,
        b"float" | b"decimal" => SyntaxShape::Decimal,
        b"filesize" => SyntaxShape::Filesize,
        b"full-cell-path" => SyntaxShape::FullCellPath,
        b"glob" => SyntaxShape::GlobPattern,
        b"int" => SyntaxShape::Int,
        b"import-pattern" => SyntaxShape::ImportPattern,
        b"keyword" => SyntaxShape::Keyword(vec![], Box::new(SyntaxShape::Any)),
        _ if bytes.starts_with(b"list") => parse_list_shape(working_set, bytes, span),
        b"math" => SyntaxShape::MathExpression,
        b"nothing" => SyntaxShape::Nothing,
        b"number" => SyntaxShape::Number,
        b"one-of" => SyntaxShape::OneOf(vec![]),
        b"operator" => SyntaxShape::Operator,
        b"path" => SyntaxShape::Filepath,
        b"range" => SyntaxShape::Range,
        _ if bytes.starts_with(b"record") => parse_collection_shape(working_set, bytes, span),
        b"signature" => SyntaxShape::Signature,
        b"string" => SyntaxShape::String,
        b"table" => SyntaxShape::Table,
        b"variable" => SyntaxShape::Variable,
        b"var-with-opt-type" => SyntaxShape::VarWithOptType,
        _ => {
            if bytes.contains(&b'@') {
                let split: Vec<_> = bytes.split(|b| b == &b'@').collect();

                let shape_span = Span::new(span.start, span.start + split[0].len());
                let cmd_span = Span::new(span.start + split[0].len() + 1, span.end);
                let shape = parse_shape_name(working_set, split[0], shape_span);

                let command_name = trim_quotes(split[1]);

                if command_name.is_empty() {
                    working_set.error(ParseError::Expected("a command name".into(), cmd_span));
                    return SyntaxShape::Any;
                }

                let decl_id = working_set.find_decl(command_name, &Type::Any);

                if let Some(decl_id) = decl_id {
                    return SyntaxShape::Custom(Box::new(shape), decl_id);
                } else {
                    working_set.error(ParseError::UnknownCommand(cmd_span));
                    return shape;
                }
            } else {
                working_set.error(ParseError::UnknownType(span));
                return SyntaxShape::Any;
            }
        }
    };

    result
}

fn parse_collection_shape(
    working_set: &mut StateWorkingSet,
    bytes: &[u8],
    span: Span,
) -> SyntaxShape {
    assert!(bytes.starts_with(b"record"));
    let name = "record";
    let mk_shape = SyntaxShape::Record;

    if bytes == name.as_bytes() {
        mk_shape(vec![])
    } else if bytes.starts_with(b"record<") {
        let Some(inner_span) = prepare_inner_span(working_set, bytes, span, 7) else {
            return SyntaxShape::Any;
        };

        // record<> or table<>
        if inner_span.end - inner_span.start == 0 {
            return mk_shape(vec![]);
        }
        let source = working_set.get_span_contents(inner_span);
        let (tokens, err) = lex_signature(
            source,
            inner_span.start,
            &[b'\n', b'\r'],
            &[b':', b','],
            true,
        );

        if let Some(err) = err {
            working_set.error(err);
            // lexer errors cause issues with span overflows
            return mk_shape(vec![]);
        }

        let mut sig = vec![];
        let mut idx = 0;

        let key_error = |span| {
            ParseError::LabeledError(
                format!("`{name}` type annotations key not string"),
                "must be a string".into(),
                span,
            )
        };

        while idx < tokens.len() {
            let TokenContents::Item = tokens[idx].contents else {
                working_set.error(key_error(tokens[idx].span));
                return mk_shape(vec![])
            };

            let key_bytes = working_set.get_span_contents(tokens[idx].span).to_vec();
            if key_bytes.first().copied() == Some(b',') {
                idx += 1;
                continue;
            }

            let Some(key) = parse_value(working_set, tokens[idx].span, &SyntaxShape::String).as_string() else {
                working_set.error(key_error(tokens[idx].span));
                return mk_shape(vec![]);
            };

            // we want to allow such an annotation
            // `record<name>` where the user leaves out the type
            if idx + 1 == tokens.len() {
                sig.push((key, SyntaxShape::Any));
                break;
            } else {
                idx += 1;
            }

            let maybe_colon = working_set.get_span_contents(tokens[idx].span).to_vec();
            match maybe_colon.as_slice() {
                b":" => {
                    if idx + 1 == tokens.len() {
                        working_set.error(ParseError::Expected(
                            "type after colon".into(),
                            tokens[idx].span,
                        ));
                        break;
                    } else {
                        idx += 1;
                    }
                }
                // a key provided without a type
                b"," => {
                    idx += 1;
                    sig.push((key, SyntaxShape::Any));
                    continue;
                }
                // a key provided without a type
                _ => {
                    sig.push((key, SyntaxShape::Any));
                    continue;
                }
            }

            let shape_bytes = working_set.get_span_contents(tokens[idx].span).to_vec();
            let shape = parse_shape_name(working_set, &shape_bytes, tokens[idx].span);
            sig.push((key, shape));
            idx += 1;
        }

        mk_shape(sig)
    } else {
        working_set.error(ParseError::UnknownType(span));

        SyntaxShape::Any
    }
}

fn parse_list_shape(working_set: &mut StateWorkingSet, bytes: &[u8], span: Span) -> SyntaxShape {
    assert!(bytes.starts_with(b"list"));

    if bytes == b"list" {
        SyntaxShape::List(Box::new(SyntaxShape::Any))
    } else if bytes.starts_with(b"list<") {
        let Some(inner_span) = prepare_inner_span(working_set, bytes, span, 5) else {
            return SyntaxShape::Any;
        };

        let inner_text = String::from_utf8_lossy(working_set.get_span_contents(inner_span));
        // remove any extra whitespace, for example `list< string >` becomes `list<string>`
        let inner_bytes = inner_text.trim().as_bytes().to_vec();

        // list<>
        if inner_bytes.is_empty() {
            SyntaxShape::List(Box::new(SyntaxShape::Any))
        } else {
            let inner_sig = parse_shape_name(working_set, &inner_bytes, inner_span);

            SyntaxShape::List(Box::new(inner_sig))
        }
    } else {
        working_set.error(ParseError::UnknownType(span));

        SyntaxShape::List(Box::new(SyntaxShape::Any))
    }
}

fn prepare_inner_span(
    working_set: &mut StateWorkingSet,
    bytes: &[u8],
    span: Span,
    prefix_len: usize,
) -> Option<Span> {
    let start = span.start + prefix_len;

    if bytes.ends_with(b">") {
        let end = span.end - 1;
        Some(Span::new(start, end))
    } else if bytes.contains(&b'>') {
        let angle_start = bytes.split(|it| it == &b'>').collect::<Vec<_>>()[0].len() + 1;
        let span = Span::new(span.start + angle_start, span.end);

        working_set.error(ParseError::LabeledError(
            "Extra characters in the parameter name".into(),
            "extra characters".into(),
            span,
        ));

        None
    } else {
        working_set.error(ParseError::Unclosed(">".into(), span));
        None
    }
}

pub fn parse_type(_working_set: &StateWorkingSet, bytes: &[u8]) -> Type {
    match bytes {
        b"binary" => Type::Binary,
        b"block" => Type::Block,
        b"bool" => Type::Bool,
        b"cellpath" => Type::CellPath,
        b"closure" => Type::Closure,
        b"date" => Type::Date,
        b"duration" => Type::Duration,
        b"error" => Type::Error,
        b"filesize" => Type::Filesize,
        b"float" | b"decimal" => Type::Float,
        b"int" => Type::Int,
        b"list" => Type::List(Box::new(Type::Any)),
        b"number" => Type::Number,
        b"range" => Type::Range,
        b"record" => Type::Record(vec![]),
        b"string" => Type::String,
        b"table" => Type::Table(vec![]), //FIXME

        _ => Type::Any,
    }
}

pub fn parse_import_pattern(working_set: &mut StateWorkingSet, spans: &[Span]) -> Expression {
    let Some(head_span) = spans.get(0) else {
        working_set.error(ParseError::WrongImportPattern(span(spans)));
        return garbage(span(spans));
    };

    let head_expr = parse_value(working_set, *head_span, &SyntaxShape::Any);

    let (maybe_module_id, head_name) = match eval_constant(working_set, &head_expr) {
        Ok(val) => match value_as_string(val, head_expr.span) {
            Ok(s) => (working_set.find_module(s.as_bytes()), s.into_bytes()),
            Err(err) => {
                working_set.error(err);
                return garbage(span(spans));
            }
        },
        Err(err) => {
            working_set.error(err);
            return garbage(span(spans));
        }
    };

    let (import_pattern, err) = if let Some(tail_span) = spans.get(1) {
        // FIXME: expand this to handle deeper imports once we support module imports
        let tail = working_set.get_span_contents(*tail_span);
        if tail == b"*" {
            (
                ImportPattern {
                    head: ImportPatternHead {
                        name: head_name,
                        id: maybe_module_id,
                        span: *head_span,
                    },
                    members: vec![ImportPatternMember::Glob { span: *tail_span }],
                    hidden: HashSet::new(),
                },
                None,
            )
        } else if tail.starts_with(b"[") {
            let result = parse_list_expression(working_set, *tail_span, &SyntaxShape::String);

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
                                name: head_name,
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
                            name: head_name,
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
                        name: head_name,
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
                    name: head_name,
                    id: maybe_module_id,
                    span: *head_span,
                },
                members: vec![],
                hidden: HashSet::new(),
            },
            None,
        )
    };

    if let Some(err) = err {
        working_set.error(err);
    }

    Expression {
        expr: Expr::ImportPattern(import_pattern),
        span: span(&spans[1..]),
        ty: Type::List(Box::new(Type::String)),
        custom_completion: None,
    }
}

pub fn parse_var_with_opt_type(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    spans_idx: &mut usize,
    mutable: bool,
) -> Expression {
    let bytes = working_set.get_span_contents(spans[*spans_idx]).to_vec();

    if bytes.contains(&b' ')
        || bytes.contains(&b'"')
        || bytes.contains(&b'\'')
        || bytes.contains(&b'`')
    {
        working_set.error(ParseError::VariableNotValid(spans[*spans_idx]));
        return garbage(spans[*spans_idx]);
    }

    if bytes.ends_with(b":") {
        // We end with colon, so the next span should be the type
        if *spans_idx + 1 < spans.len() {
            *spans_idx += 1;
            let type_bytes = working_set.get_span_contents(spans[*spans_idx]);

            let ty = parse_type(working_set, type_bytes);

            let var_name = bytes[0..(bytes.len() - 1)].to_vec();

            if !is_variable(&var_name) {
                working_set.error(ParseError::Expected(
                    "valid variable name".into(),
                    spans[*spans_idx],
                ));
                return garbage(spans[*spans_idx]);
            }

            let id = working_set.add_variable(var_name, spans[*spans_idx - 1], ty.clone(), mutable);

            Expression {
                expr: Expr::VarDecl(id),
                span: span(&spans[*spans_idx - 1..*spans_idx + 1]),
                ty,
                custom_completion: None,
            }
        } else {
            let var_name = bytes[0..(bytes.len() - 1)].to_vec();

            if !is_variable(&var_name) {
                working_set.error(ParseError::Expected(
                    "valid variable name".into(),
                    spans[*spans_idx],
                ));
                return garbage(spans[*spans_idx]);
            }

            let id = working_set.add_variable(var_name, spans[*spans_idx], Type::Any, mutable);

            working_set.error(ParseError::MissingType(spans[*spans_idx]));
            Expression {
                expr: Expr::VarDecl(id),
                span: spans[*spans_idx],
                ty: Type::Any,
                custom_completion: None,
            }
        }
    } else {
        let var_name = bytes;

        if !is_variable(&var_name) {
            working_set.error(ParseError::Expected(
                "valid variable name".into(),
                spans[*spans_idx],
            ));
            return garbage(spans[*spans_idx]);
        }

        let id = working_set.add_variable(
            var_name,
            span(&spans[*spans_idx..*spans_idx + 1]),
            Type::Any,
            mutable,
        );

        Expression {
            expr: Expr::VarDecl(id),
            span: span(&spans[*spans_idx..*spans_idx + 1]),
            ty: Type::Any,
            custom_completion: None,
        }
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
}

pub fn parse_row_condition(working_set: &mut StateWorkingSet, spans: &[Span]) -> Expression {
    let var_id = working_set.add_variable(b"$it".to_vec(), span(spans), Type::Any, false);
    let expression = parse_math_expression(working_set, spans, Some(var_id));
    let span = span(spans);

    let block_id = match expression.expr {
        Expr::Block(block_id) => block_id,
        Expr::Closure(block_id) => block_id,
        _ => {
            // We have an expression, so let's convert this into a block.
            let mut block = Block::new();
            let mut pipeline = Pipeline::new();
            pipeline
                .elements
                .push(PipelineElement::Expression(None, expression));

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
        working_set.error(ParseError::Expected(
            "[ or (".into(),
            Span::new(start, start + 1),
        ));
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
    #[allow(clippy::enum_variant_names)]
    enum ParseMode {
        ArgMode,
        AfterCommaArgMode,
        TypeMode,
        DefaultValueMode,
    }

    #[derive(Debug)]
    enum Arg {
        Positional(PositionalArg, bool), // bool - required
        RestPositional(PositionalArg),
        Flag(Flag),
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
    let mut parse_mode = ParseMode::ArgMode;

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
                        ParseMode::ArgMode => {
                            parse_mode = ParseMode::TypeMode;
                        }
                        ParseMode::AfterCommaArgMode => {
                            working_set
                                .error(ParseError::Expected("parameter or flag".into(), span));
                        }
                        ParseMode::TypeMode | ParseMode::DefaultValueMode => {
                            // We're seeing two types for the same thing for some reason, error
                            working_set.error(ParseError::Expected("type".into(), span));
                        }
                    }
                }
                // The = symbol separates a variable from its default value
                else if contents == b"=" {
                    match parse_mode {
                        ParseMode::ArgMode | ParseMode::TypeMode => {
                            parse_mode = ParseMode::DefaultValueMode;
                        }
                        ParseMode::AfterCommaArgMode => {
                            working_set
                                .error(ParseError::Expected("parameter or flag".into(), span));
                        }
                        ParseMode::DefaultValueMode => {
                            // We're seeing two default values for some reason, error
                            working_set.error(ParseError::Expected("default value".into(), span));
                        }
                    }
                }
                // The , symbol separates params only
                else if contents == b"," {
                    match parse_mode {
                        ParseMode::ArgMode => parse_mode = ParseMode::AfterCommaArgMode,
                        ParseMode::AfterCommaArgMode => {
                            working_set
                                .error(ParseError::Expected("parameter or flag".into(), span));
                        }
                        ParseMode::TypeMode => {
                            working_set.error(ParseError::Expected("type".into(), span));
                        }
                        ParseMode::DefaultValueMode => {
                            working_set.error(ParseError::Expected("default value".into(), span));
                        }
                    }
                } else {
                    match parse_mode {
                        ParseMode::ArgMode | ParseMode::AfterCommaArgMode => {
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
                                        "valid variable name for this long flag".into(),
                                        span,
                                    ))
                                }

                                let var_id =
                                    working_set.add_variable(variable_name, span, Type::Any, false);

                                // If there's no short flag, exit now. Otherwise, parse it.
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
                                    working_set.error(ParseError::Expected(
                                        "only one short flag alternative".into(),
                                        span,
                                    ));
                                } else {
                                    let short_flag = &flags[1];
                                    let short_flag = if !short_flag.starts_with(b"-")
                                        || !short_flag.ends_with(b")")
                                    {
                                        working_set.error(ParseError::Expected(
                                            "short flag alternative for the long flag".into(),
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
                                            "valid variable name for this short flag".into(),
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
                                        working_set
                                            .error(ParseError::Expected("short flag".into(), span));
                                    }
                                }
                                parse_mode = ParseMode::ArgMode;
                            }
                            // Mandatory short flag, e.g. -e (must be one character)
                            else if contents.starts_with(b"-") && contents.len() > 1 {
                                let short_flag = &contents[1..];
                                let short_flag = String::from_utf8_lossy(short_flag).to_string();
                                let chars: Vec<char> = short_flag.chars().collect();

                                if chars.len() > 1 {
                                    working_set
                                        .error(ParseError::Expected("short flag".into(), span));
                                }

                                let mut encoded_var_name = vec![0u8; 4];
                                let len = chars[0].encode_utf8(&mut encoded_var_name).len();
                                let variable_name = encoded_var_name[0..len].to_vec();

                                if !is_variable(&variable_name) {
                                    working_set.error(ParseError::Expected(
                                        "valid variable name for this short flag".into(),
                                        span,
                                    ))
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
                                parse_mode = ParseMode::ArgMode;
                            }
                            // Short flag alias for long flag, e.g. --b (-a)
                            // This is the same as the short flag in --b(-a)
                            else if contents.starts_with(b"(-") {
                                if matches!(parse_mode, ParseMode::AfterCommaArgMode) {
                                    working_set.error(ParseError::Expected(
                                        "parameter or flag".into(),
                                        span,
                                    ));
                                }
                                let short_flag = &contents[2..];

                                let short_flag = if !short_flag.ends_with(b")") {
                                    working_set
                                        .error(ParseError::Expected("short flag".into(), span));
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
                                                working_set.error(ParseError::Expected(
                                                    "one short flag".into(),
                                                    span,
                                                ));
                                            } else {
                                                flag.short = Some(chars[0]);
                                            }
                                        }
                                        _ => {
                                            working_set.error(ParseError::Expected(
                                                "unknown flag".into(),
                                                span,
                                            ));
                                        }
                                    }
                                } else {
                                    working_set
                                        .error(ParseError::Expected("short flag".into(), span));
                                }
                            }
                            // Positional arg, optional
                            else if contents.ends_with(b"?") {
                                let contents: Vec<_> = contents[..(contents.len() - 1)].into();
                                let name = String::from_utf8_lossy(&contents).to_string();

                                if !is_variable(&contents) {
                                    working_set.error(ParseError::Expected(
                                        "valid variable name for this optional parameter".into(),
                                        span,
                                    ))
                                }

                                let var_id =
                                    working_set.add_variable(contents, span, Type::Any, false);

                                args.push(Arg::Positional(
                                    PositionalArg {
                                        desc: String::new(),
                                        name,
                                        shape: SyntaxShape::Any,
                                        var_id: Some(var_id),
                                        default_value: None,
                                    },
                                    false,
                                ));
                                parse_mode = ParseMode::ArgMode;
                            }
                            // Rest param
                            else if let Some(contents) = contents.strip_prefix(b"...") {
                                let name = String::from_utf8_lossy(contents).to_string();
                                let contents_vec: Vec<u8> = contents.to_vec();

                                if !is_variable(&contents_vec) {
                                    working_set.error(ParseError::Expected(
                                        "valid variable name for this rest parameter".into(),
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
                                parse_mode = ParseMode::ArgMode;
                            }
                            // Normal param
                            else {
                                let name = String::from_utf8_lossy(&contents).to_string();
                                let contents_vec = contents.to_vec();

                                if !is_variable(&contents_vec) {
                                    working_set.error(ParseError::Expected(
                                        "valid variable name for this parameter".into(),
                                        span,
                                    ))
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
                                ));
                                parse_mode = ParseMode::ArgMode;
                            }
                        }
                        ParseMode::TypeMode => {
                            if let Some(last) = args.last_mut() {
                                let syntax_shape = parse_shape_name(working_set, &contents, span);
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
                                let expression = parse_value(working_set, span, &SyntaxShape::Any);

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

                                        *shape = expression.ty.to_shape();
                                        *required = false;
                                    }
                                    Arg::RestPositional(..) => {
                                        working_set.error(ParseError::AssignmentMismatch(
                                            "Rest parameter was given a default value".into(),
                                            "can't have default value".into(),
                                            expression.span,
                                        ))
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
                let contents = working_set.get_span_contents(Span::new(span.start + 1, span.end));

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
            Arg::Flag(flag) => sig.named.push(flag),
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

    let (output, err) = lite_parse(&output);
    if let Some(err) = err {
        working_set.error(err)
    }

    let mut args = vec![];

    let mut contained_type: Option<Type> = None;

    if !output.block.is_empty() {
        for arg in &output.block[0].commands {
            let mut spans_idx = 0;

            if let LiteElement::Command(_, command) = arg {
                while spans_idx < command.parts.len() {
                    let arg = parse_multispan_value(
                        working_set,
                        &command.parts,
                        &mut spans_idx,
                        element_shape,
                    );

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

pub fn parse_table_expression(
    working_set: &mut StateWorkingSet,
    original_span: Span,
) -> Expression {
    let bytes = working_set.get_span_contents(original_span);

    let mut start = original_span.start;
    let mut end = original_span.end;

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

    let (output, err) = lex(source, start, &[b'\n', b'\r', b','], &[], true);
    if let Some(err) = err {
        working_set.error(err);
    }

    let (output, err) = lite_parse(&output);
    if let Some(err) = err {
        working_set.error(err);
    }

    match output.block.len() {
        0 => Expression {
            expr: Expr::List(vec![]),
            span: original_span,
            ty: Type::List(Box::new(Type::Any)),
            custom_completion: None,
        },
        1 => {
            // List
            parse_list_expression(working_set, original_span, &SyntaxShape::Any)
        }
        _ => {
            match &output.block[0].commands[0] {
                LiteElement::Command(_, command)
                | LiteElement::Redirection(_, _, command)
                | LiteElement::SeparateRedirection {
                    out: (_, command), ..
                } => {
                    let mut table_headers = vec![];

                    let headers = parse_value(
                        working_set,
                        command.parts[0],
                        &SyntaxShape::List(Box::new(SyntaxShape::Any)),
                    );

                    if let Expression {
                        expr: Expr::List(headers),
                        ..
                    } = headers
                    {
                        table_headers = headers;
                    }

                    match &output.block[1].commands[0] {
                        LiteElement::Command(_, command)
                        | LiteElement::Redirection(_, _, command)
                        | LiteElement::SeparateRedirection {
                            out: (_, command), ..
                        } => {
                            let mut rows = vec![];
                            for part in &command.parts {
                                let values = parse_value(
                                    working_set,
                                    *part,
                                    &SyntaxShape::List(Box::new(SyntaxShape::Any)),
                                );
                                if let Expression {
                                    expr: Expr::List(values),
                                    span,
                                    ..
                                } = values
                                {
                                    match values.len().cmp(&table_headers.len()) {
                                        std::cmp::Ordering::Less => working_set.error(
                                            ParseError::MissingColumns(table_headers.len(), span),
                                        ),
                                        std::cmp::Ordering::Equal => {}
                                        std::cmp::Ordering::Greater => {
                                            working_set.error(ParseError::ExtraColumns(
                                                table_headers.len(),
                                                values[table_headers.len()].span,
                                            ))
                                        }
                                    }

                                    rows.push(values);
                                }
                            }

                            Expression {
                                expr: Expr::Table(table_headers, rows),
                                span: original_span,
                                ty: Type::Table(vec![]), //FIXME
                                custom_completion: None,
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn parse_block_expression(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: block expression");

    let bytes = working_set.get_span_contents(span);

    let mut start = span.start;
    let mut end = span.end;

    if bytes.starts_with(b"{") {
        start += 1;
    } else {
        working_set.error(ParseError::Expected("block".into(), span));
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
            working_set.error(ParseError::Expected(
                "block but found closure".into(),
                *span,
            ));
            (None, 0)
        }
        _ => (None, 0),
    };

    let mut output = parse_block(working_set, &output[amt_to_skip..], span, false, false);

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
        working_set.error(ParseError::Expected("closure".into(), span));
        return garbage(span);
    }
    if bytes.ends_with(b"}") {
        end -= 1;
    } else {
        working_set.error(ParseError::Unclosed("}".into(), Span::new(end, end)));
    }

    let inner_span = Span::new(start, end);

    let source = working_set.get_span_contents(inner_span);

    let (output, err) = lex(source, start, &[b' ', b'\r', b'\n', b',', b'|'], &[], false);
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

        // Multiple patterns connected by '|'
        let mut connector = working_set.get_span_contents(output[position].span);
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
                span: Span::new(start, end),
            }
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
        working_set.error(ParseError::Expected("closure".into(), span));
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
                working_set.error(ParseError::Expected(
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
                working_set.error(ParseError::Expected("non-boolean value".into(), span));
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
                working_set.error(ParseError::Expected("non-boolean value".into(), span));
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

    if matches!(shape, SyntaxShape::MatchPattern) {
        return parse_match_pattern(working_set, span);
    }

    match bytes[0] {
        b'$' => return parse_dollar_expr(working_set, span),
        b'(' => return parse_paren_expr(working_set, span, shape),
        b'{' => return parse_brace_expr(working_set, span, shape),
        b'[' => match shape {
            SyntaxShape::Any
            | SyntaxShape::List(_)
            | SyntaxShape::Table
            | SyntaxShape::Signature => {}
            _ => {
                working_set.error(ParseError::Expected("non-[] value".into(), span));
                return Expression::garbage(span);
            }
        },
        _ => {}
    }

    match shape {
        SyntaxShape::Custom(shape, custom_completion) => {
            let mut expression = parse_value(working_set, span, shape);
            expression.custom_completion = Some(*custom_completion);
            expression
        }
        SyntaxShape::Number => parse_number(working_set, span),
        SyntaxShape::Decimal => parse_float(working_set, span),
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
        SyntaxShape::MatchPattern => parse_match_pattern(working_set, span),
        SyntaxShape::Signature => {
            if bytes.starts_with(b"[") {
                parse_signature(working_set, span)
            } else {
                working_set.error(ParseError::Expected("signature".into(), span));

                Expression::garbage(span)
            }
        }
        SyntaxShape::List(elem) => {
            if bytes.starts_with(b"[") {
                parse_list_expression(working_set, span, elem)
            } else {
                working_set.error(ParseError::Expected("list".into(), span));

                Expression::garbage(span)
            }
        }
        SyntaxShape::Table => {
            if bytes.starts_with(b"[") {
                parse_table_expression(working_set, span)
            } else {
                working_set.error(ParseError::Expected("table".into(), span));

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
                working_set.error(ParseError::Expected("bool".into(), span));

                Expression::garbage(span)
            }
        }

        // Be sure to return ParseError::Expected(..) if invoked for one of these shapes, but lex
        // stream doesn't start with '{'} -- parsing in SyntaxShape::Any arm depends on this error variant.
        SyntaxShape::Block | SyntaxShape::Closure(..) | SyntaxShape::Record(_) => {
            working_set.error(ParseError::Expected(
                "block, closure or record".into(),
                span,
            ));

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
                    SyntaxShape::DateTime, //FIXME requires 3 failed conversion attempts before failing
                    SyntaxShape::Record(vec![]),
                    SyntaxShape::Closure(None),
                    SyntaxShape::Block,
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
                            Some(ParseError::Expected(_, _)) => {
                                working_set.parse_errors.truncate(starting_error_count);
                                continue;
                            }
                            _ => {
                                return s;
                            }
                        }
                    }
                }
                working_set.error(ParseError::Expected("any shape".into(), span));
                garbage(span)
            }
        }
        x => {
            working_set.error(ParseError::Expected(x.to_type().to_string(), span));
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
            working_set.error(ParseError::Expected("operator".into(), span));
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

    if first_span == b"if" || first_span == b"match" {
        // If expression
        if spans.len() > 1 {
            return parse_call(working_set, spans, spans[0], false);
        } else {
            working_set.error(ParseError::Expected(
                "expression".into(),
                Span::new(spans[0].end, spans[0].end),
            ));
            return garbage(spans[0]);
        }
    } else if first_span == b"not" {
        if spans.len() > 1 {
            let remainder = parse_math_expression(working_set, &spans[1..], lhs_row_var_id);
            return Expression {
                expr: Expr::UnaryNot(Box::new(remainder)),
                span: span(spans),
                ty: Type::Bool,
                custom_completion: None,
            };
        } else {
            working_set.error(ParseError::Expected(
                "expression".into(),
                Span::new(spans[0].end, spans[0].end),
            ));
            return garbage(spans[0]);
        }
    }

    let mut lhs = parse_value(working_set, spans[0], &SyntaxShape::Any);
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

        let rhs = parse_value(working_set, spans[idx], &SyntaxShape::Any);

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
            expand_to_cell_path(working_set, &mut lhs, row_var_id);
        }

        let (result_ty, err) = math_result_type(working_set, &mut lhs, &mut op, &mut rhs);
        if let Some(err) = err {
            working_set.error(err)
        }

        let binary_op_span = span(&[lhs.span, rhs.span]);
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

pub fn parse_expression(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    is_subexpression: bool,
) -> Expression {
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
        return garbage(span(spans));
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

                parse_call(working_set, &spans[pos..], spans[0], is_subexpression)
            }
            b"let" | b"const" | b"mut" => {
                working_set.error(ParseError::AssignInPipeline(
                    String::from_utf8(bytes)
                        .expect("builtin commands bytes should be able to convert to string"),
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
                ));
                parse_call(working_set, &spans[pos..], spans[0], is_subexpression)
            }
            b"overlay" => {
                if spans.len() > 1 && working_set.get_span_contents(spans[1]) == b"list" {
                    // whitelist 'overlay list'
                    parse_call(working_set, &spans[pos..], spans[0], is_subexpression)
                } else {
                    working_set.error(ParseError::BuiltinCommandInPipeline(
                        "overlay".into(),
                        spans[0],
                    ));

                    parse_call(working_set, &spans[pos..], spans[0], is_subexpression)
                }
            }
            b"where" => parse_where_expr(working_set, &spans[pos..]),
            #[cfg(feature = "plugin")]
            b"register" => {
                working_set.error(ParseError::BuiltinCommandInPipeline(
                    "plugin".into(),
                    spans[0],
                ));

                parse_call(working_set, &spans[pos..], spans[0], is_subexpression)
            }

            _ => parse_call(working_set, &spans[pos..], spans[0], is_subexpression),
        }
    };

    let with_env = working_set.find_decl(b"with-env", &Type::Any);

    if !shorthand.is_empty() {
        if let Some(decl_id) = with_env {
            let mut block = Block::default();
            let ty = output.ty.clone();
            block.pipelines = vec![Pipeline::from_vec(vec![output])];

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
                head: Span::unknown(),
                decl_id,
                arguments,
                redirect_stdout: true,
                redirect_stderr: false,
                parser_info: HashMap::new(),
            }));

            Expression {
                expr,
                custom_completion: None,
                span: span(spans),
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
        if let Some(var_id) = working_set.find_variable(bytes) {
            let input = working_set.get_variable(var_id).ty.clone();
            working_set.type_scope.add_type(input);

            Some(var_id)
        } else {
            None
        }
    } else {
        working_set.error(ParseError::Expected("valid variable name".into(), span));

        None
    }
}

pub fn parse_builtin_commands(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
    is_subexpression: bool,
) -> Pipeline {
    if !is_math_expression_like(working_set, lite_command.parts[0])
        && !is_unaliasable_parser_keyword(working_set, &lite_command.parts)
    {
        let name = working_set.get_span_contents(lite_command.parts[0]);
        if let Some(decl_id) = working_set.find_decl(name, &Type::Any) {
            let cmd = working_set.get_decl(decl_id);
            if cmd.is_alias() {
                // Parse keywords that can be aliased. Note that we check for "unaliasable" keywords
                // because alias can have any name, therefore, we can't check for "aliasable" keywords.
                let call_expr = parse_call(
                    working_set,
                    &lite_command.parts,
                    lite_command.parts[0],
                    is_subexpression,
                );

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

    let name = working_set.get_span_contents(lite_command.parts[0]);

    match name {
        b"def" | b"def-env" => parse_def(working_set, lite_command, None),
        b"extern" => parse_extern(working_set, lite_command, None),
        b"let" | b"const" => parse_let_or_const(working_set, &lite_command.parts),
        b"mut" => parse_mut(working_set, &lite_command.parts),
        b"for" => {
            let expr = parse_for(working_set, &lite_command.parts);
            Pipeline::from_vec(vec![expr])
        }
        b"alias" => parse_alias(working_set, lite_command, None),
        b"module" => parse_module(working_set, lite_command),
        b"use" => {
            let (pipeline, _) = parse_use(working_set, &lite_command.parts);
            pipeline
        }
        b"overlay" => parse_keyword(working_set, lite_command, is_subexpression),
        b"source" | b"source-env" => parse_source(working_set, &lite_command.parts),
        b"export" => parse_export_in_block(working_set, lite_command),
        b"hide" => parse_hide(working_set, &lite_command.parts),
        b"where" => parse_where(working_set, &lite_command.parts),
        #[cfg(feature = "plugin")]
        b"register" => parse_register(working_set, &lite_command.parts),
        _ => {
            let expr = parse_expression(working_set, &lite_command.parts, is_subexpression);

            Pipeline::from_vec(vec![expr])
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
        working_set.error(ParseError::Expected(
            "{".into(),
            Span::new(start, start + 1),
        ));
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
        let field = parse_value(working_set, tokens[idx].span, &SyntaxShape::Any);

        idx += 1;
        if idx == tokens.len() {
            working_set.error(ParseError::Expected("record".into(), span));
            return garbage(span);
        }
        let colon = working_set.get_span_contents(tokens[idx].span);
        idx += 1;
        if idx == tokens.len() || colon != b":" {
            //FIXME: need better error
            working_set.error(ParseError::Expected("record".into(), span));
            return garbage(span);
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
        output.push((field, value));
    }

    Expression {
        expr: Expr::Record(output),
        span,
        ty: (if let Some(fields) = field_types {
            Type::Record(fields)
        } else {
            Type::Any
        }),
        custom_completion: None,
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
    working_set.type_scope.enter_scope();

    // Pre-declare any definition so that definitions
    // that share the same block can see each other
    for pipeline in &lite_block.block {
        if pipeline.commands.len() == 1 {
            match &pipeline.commands[0] {
                LiteElement::Command(_, command)
                | LiteElement::Redirection(_, _, command)
                | LiteElement::SeparateRedirection {
                    out: (_, command), ..
                } => parse_def_predecl(working_set, &command.parts),
            }
        }
    }

    let mut block = Block::new_with_capacity(lite_block.block.len());

    for (idx, pipeline) in lite_block.block.iter().enumerate() {
        if pipeline.commands.len() > 1 {
            let mut output = pipeline
                .commands
                .iter()
                .map(|command| match command {
                    LiteElement::Command(span, command) => {
                        trace!("parsing: pipeline element: command");
                        let expr = parse_expression(working_set, &command.parts, is_subexpression);
                        working_set.type_scope.add_type(expr.ty.clone());

                        PipelineElement::Expression(*span, expr)
                    }
                    LiteElement::Redirection(span, redirection, command) => {
                        trace!("parsing: pipeline element: redirection");
                        let expr = parse_string(working_set, command.parts[0]);

                        working_set.type_scope.add_type(expr.ty.clone());

                        PipelineElement::Redirection(*span, redirection.clone(), expr)
                    }
                    LiteElement::SeparateRedirection {
                        out: (out_span, out_command),
                        err: (err_span, err_command),
                    } => {
                        trace!("parsing: pipeline element: separate redirection");
                        let out_expr = parse_string(working_set, out_command.parts[0]);

                        working_set.type_scope.add_type(out_expr.ty.clone());

                        let err_expr = parse_string(working_set, err_command.parts[0]);

                        working_set.type_scope.add_type(err_expr.ty.clone());

                        PipelineElement::SeparateRedirection {
                            out: (*out_span, out_expr),
                            err: (*err_span, err_expr),
                        }
                    }
                })
                .collect::<Vec<PipelineElement>>();

            if is_subexpression {
                for element in output.iter_mut().skip(1) {
                    if element.has_in_variable(working_set) {
                        *element = wrap_element_with_collect(working_set, element);
                    }
                }
            } else {
                for element in output.iter_mut() {
                    if element.has_in_variable(working_set) {
                        *element = wrap_element_with_collect(working_set, element);
                    }
                }
            }

            block.pipelines.push(Pipeline { elements: output })
        } else {
            match &pipeline.commands[0] {
                LiteElement::Command(_, command)
                | LiteElement::Redirection(_, _, command)
                | LiteElement::SeparateRedirection {
                    out: (_, command), ..
                } => {
                    let mut pipeline =
                        parse_builtin_commands(working_set, command, is_subexpression);

                    if idx == 0 {
                        if let Some(let_decl_id) = working_set.find_decl(b"let", &Type::Any) {
                            if let Some(let_env_decl_id) =
                                working_set.find_decl(b"let-env", &Type::Any)
                            {
                                for element in pipeline.elements.iter_mut() {
                                    if let PipelineElement::Expression(
                                        _,
                                        Expression {
                                            expr: Expr::Call(call),
                                            ..
                                        },
                                    ) = element
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
                                        } else if element.has_in_variable(working_set)
                                            && !is_subexpression
                                        {
                                            *element =
                                                wrap_element_with_collect(working_set, element);
                                        }
                                    } else if element.has_in_variable(working_set)
                                        && !is_subexpression
                                    {
                                        *element = wrap_element_with_collect(working_set, element);
                                    }
                                }
                            }
                        }
                    }

                    block.pipelines.push(pipeline)
                }
            }
        }
    }

    if scoped {
        working_set.exit_scope();
    }
    working_set.type_scope.exit_scope();

    block.span = Some(span);

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
    match element {
        PipelineElement::Expression(_, expression)
        | PipelineElement::Redirection(_, _, expression)
        | PipelineElement::And(_, expression)
        | PipelineElement::Or(_, expression) => {
            discover_captures_in_expr(working_set, expression, seen, seen_blocks, output)
        }
        PipelineElement::SeparateRedirection {
            out: (_, out_expr),
            err: (_, err_expr),
        } => {
            discover_captures_in_expr(working_set, out_expr, seen, seen_blocks, output)?;
            discover_captures_in_expr(working_set, err_expr, seen, seen_blocks, output)?;
            Ok(())
        }
    }
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

                            let mut result = vec![];
                            discover_captures_in_closure(
                                working_set,
                                block,
                                &mut seen,
                                seen_blocks,
                                &mut result,
                            )?;
                            output.extend(&result);
                            seen_blocks.insert(block_id, result);
                        }
                    }
                }
            }

            for named in call.named_iter() {
                if let Some(arg) = &named.2 {
                    discover_captures_in_expr(working_set, arg, seen, seen_blocks, output)?;
                }
            }

            for positional in call.positional_iter() {
                discover_captures_in_expr(working_set, positional, seen, seen_blocks, output)?;
            }
        }
        Expr::CellPath(_) => {}
        Expr::DateTime(_) => {}
        Expr::ExternalCall(head, exprs, _) => {
            discover_captures_in_expr(working_set, head, seen, seen_blocks, output)?;

            for expr in exprs {
                discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
            }
        }
        Expr::Filepath(_) => {}
        Expr::Directory(_) => {}
        Expr::Float(_) => {}
        Expr::FullCellPath(cell_path) => {
            discover_captures_in_expr(working_set, &cell_path.head, seen, seen_blocks, output)?;
        }
        Expr::ImportPattern(_) => {}
        Expr::Overlay(_) => {}
        Expr::Garbage => {}
        Expr::Nothing => {}
        Expr::GlobPattern(_) => {}
        Expr::Int(_) => {}
        Expr::Keyword(_, _, expr) => {
            discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
        }
        Expr::List(exprs) => {
            for expr in exprs {
                discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
            }
        }
        Expr::Operator(_) => {}
        Expr::Range(expr1, expr2, expr3, _) => {
            if let Some(expr) = expr1 {
                discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
            }
            if let Some(expr) = expr2 {
                discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
            }
            if let Some(expr) = expr3 {
                discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
            }
        }
        Expr::Record(fields) => {
            for (field_name, field_value) in fields {
                discover_captures_in_expr(working_set, field_name, seen, seen_blocks, output)?;
                discover_captures_in_expr(working_set, field_value, seen, seen_blocks, output)?;
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
                discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
            }
        }
        Expr::MatchPattern(_) => {}
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
        Expr::Table(headers, values) => {
            for header in headers {
                discover_captures_in_expr(working_set, header, seen, seen_blocks, output)?;
            }
            for row in values {
                for cell in row {
                    discover_captures_in_expr(working_set, cell, seen, seen_blocks, output)?;
                }
            }
        }
        Expr::ValueWithUnit(expr, _) => {
            discover_captures_in_expr(working_set, expr, seen, seen_blocks, output)?;
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

fn wrap_element_with_collect(
    working_set: &mut StateWorkingSet,
    element: &PipelineElement,
) -> PipelineElement {
    match element {
        PipelineElement::Expression(span, expression) => {
            PipelineElement::Expression(*span, wrap_expr_with_collect(working_set, expression))
        }
        PipelineElement::Redirection(span, redirection, expression) => {
            PipelineElement::Redirection(
                *span,
                redirection.clone(),
                wrap_expr_with_collect(working_set, expression),
            )
        }
        PipelineElement::SeparateRedirection {
            out: (out_span, out_exp),
            err: (err_span, err_exp),
        } => PipelineElement::SeparateRedirection {
            out: (*out_span, wrap_expr_with_collect(working_set, out_exp)),
            err: (*err_span, wrap_expr_with_collect(working_set, err_exp)),
        },
        PipelineElement::And(span, expression) => {
            PipelineElement::And(*span, wrap_expr_with_collect(working_set, expression))
        }
        PipelineElement::Or(span, expression) => {
            PipelineElement::Or(*span, wrap_expr_with_collect(working_set, expression))
        }
    }
}

fn wrap_expr_with_collect(working_set: &mut StateWorkingSet, expr: &Expression) -> Expression {
    let span = expr.span;

    if let Some(decl_id) = working_set.find_decl(b"collect", &Type::Any) {
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

        let mut expr = expr.clone();
        expr.replace_in_variable(working_set, var_id);

        let block = Block {
            pipelines: vec![Pipeline::from_vec(vec![expr])],
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
                parser_info: HashMap::new(),
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
) -> Block {
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
            return block;
        } else {
            let (output, err) = lex(contents, new_span.start, &[], &[], false);
            if let Some(err) = err {
                working_set.error(err)
            }

            parse_block(working_set, &output, new_span, scoped, false)
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
        Ok(_) => output.captures = captures.into_iter().map(|(var_id, _)| var_id).collect(),
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
        if !captures.is_empty() && block_captures_empty {
            let block = working_set.get_block_mut(block_id);
            block.captures = captures.into_iter().map(|(var_id, _)| var_id).collect();
        }
    }

    output
}
