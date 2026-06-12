use crate::{
    lite_parser::LiteCommand,
    parse_helpers::{PERCENT_FORCED_BUILTIN_PARSER_INFO, garbage},
    parse_source::find_dirs_var,
    type_check::type_compatible,
};
use log::trace;
use nu_engine::DIR_VAR_PARSER_INFO;
use nu_protocol::{
    DeclId, Flag, ParseError, PositionalArg, ShellError, Signature, Span, Spanned, SyntaxShape,
    Type,
    ast::*,
    did_you_mean,
    engine::{CommandType, StateWorkingSet},
};
use std::str;

/// Return type of `check_call`
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CallKind {
    Help,
    Valid,
    Invalid,
}

pub(crate) fn check_call(
    working_set: &mut StateWorkingSet,
    command: Span,
    sig: &Signature,
    call: &Call,
) -> CallKind {
    // Allow the call to pass if they pass in the help flag
    if call.named_iter().any(|(n, _, _)| n.item == "help") {
        return CallKind::Help;
    }

    if call.positional_iter().count() < sig.required_positional.len() {
        let end_offset = call
            .positional_iter()
            .last()
            .map(|last| last.span.end)
            .unwrap_or(command.end);
        // Comparing the types of all signature positional arguments against the parsed
        // expressions found in the call. If one type is not found then it could be assumed
        // that positional argument is missing from the parsed call
        for argument in &sig.required_positional {
            let found = call.positional_iter().fold(false, |ac, expr| {
                if argument.shape.to_type() == expr.ty || argument.shape == SyntaxShape::Any {
                    true
                } else {
                    ac
                }
            });
            if !found {
                working_set.error(ParseError::MissingPositional(
                    argument.name.clone(),
                    Span::new(end_offset, end_offset),
                    sig.call_signature(),
                ));
                return CallKind::Invalid;
            }
        }

        let missing = &sig.required_positional[call.positional_iter().count()];
        working_set.error(ParseError::MissingPositional(
            missing.name.clone(),
            Span::new(end_offset, end_offset),
            sig.call_signature(),
        ));
        return CallKind::Invalid;
    } else {
        for req_flag in sig.named.iter().filter(|x| x.required) {
            if call.named_iter().all(|(n, _, _)| n.item != req_flag.long) {
                working_set.error(ParseError::MissingRequiredFlag(
                    req_flag.long.clone(),
                    command,
                ));
                return CallKind::Invalid;
            }
        }
    }
    CallKind::Valid
}

fn parse_unknown_arg(
    working_set: &mut StateWorkingSet,
    span: Span,
    signature: &Signature,
) -> Expression {
    let shape = signature
        .rest_positional
        .as_ref()
        .map(|arg| arg.shape.clone())
        .unwrap_or(SyntaxShape::Any);

    crate::parser::parse_value(working_set, span, &shape)
}

fn parse_external_string(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let contents = working_set.get_span_contents(span);

    if contents.starts_with(b"r#") {
        crate::parser::parse_raw_string(working_set, span)
    } else if contents
        .iter()
        .any(|b| matches!(b, b'"' | b'\'' | b'(' | b')' | b'`'))
    {
        enum State {
            Bare {
                from: usize,
            },
            BackTickQuote {
                from: usize,
            },
            Quote {
                from: usize,
                quote_char: u8,
                escaped: bool,
            },
            Parenthesized {
                from: usize,
                depth: usize,
            },
        }
        // Find the spans of parts of the string that can be parsed as their own strings for
        // concatenation.
        //
        // By passing each of these parts to `parse_string()`, we can eliminate the quotes and also
        // handle string interpolation.
        let make_span = |from: usize, index: usize| Span {
            start: span.start + from,
            end: span.start + index,
        };
        let mut spans = vec![];
        let mut state = State::Bare { from: 0 };
        let mut index = 0;
        while index < contents.len() {
            let ch = contents[index];
            match &mut state {
                State::Bare { from } => match ch {
                    b'"' | b'\'' => {
                        // Push bare string
                        if index != *from {
                            spans.push(make_span(*from, index));
                        }
                        // then transition to other state
                        state = State::Quote {
                            from: index,
                            quote_char: ch,
                            escaped: false,
                        };
                    }
                    b'$' => {
                        if let Some(&quote_char @ (b'"' | b'\'')) = contents.get(index + 1) {
                            // Start a dollar quote (interpolated string)
                            if index != *from {
                                spans.push(make_span(*from, index));
                            }
                            state = State::Quote {
                                from: index,
                                quote_char,
                                escaped: false,
                            };
                            // Skip over two chars (the dollar sign and the quote)
                            index += 2;
                            continue;
                        }
                    }
                    b'`' => {
                        if index != *from {
                            spans.push(make_span(*from, index))
                        }
                        state = State::BackTickQuote { from: index }
                    }
                    b'(' => {
                        if index != *from {
                            spans.push(make_span(*from, index))
                        }
                        state = State::Parenthesized {
                            from: index,
                            depth: 1,
                        }
                    }
                    // Continue to consume
                    _ => (),
                },
                State::Quote {
                    from,
                    quote_char,
                    escaped,
                } => match ch {
                    ch if ch == *quote_char && !*escaped => {
                        // quoted string ended, just make a new span for it.
                        spans.push(make_span(*from, index + 1));
                        // go back to Bare state.
                        state = State::Bare { from: index + 1 };
                    }
                    b'\\' if !*escaped && *quote_char == b'"' => {
                        // The next token is escaped so it doesn't count (only for double quote)
                        *escaped = true;
                    }
                    _ => {
                        *escaped = false;
                    }
                },
                State::BackTickQuote { from } => {
                    if ch == b'`' {
                        spans.push(make_span(*from, index + 1));
                        state = State::Bare { from: index + 1 };
                    }
                }
                State::Parenthesized { from, depth } => {
                    if ch == b')' {
                        if *depth == 1 {
                            spans.push(make_span(*from, index + 1));
                            state = State::Bare { from: index + 1 };
                        } else {
                            *depth -= 1;
                        }
                    } else if ch == b'(' {
                        *depth += 1;
                    }
                }
            }
            index += 1;
        }

        // Add the final span
        match state {
            State::Bare { from }
            | State::Quote { from, .. }
            | State::Parenthesized { from, .. }
            | State::BackTickQuote { from, .. } => {
                if from < contents.len() {
                    spans.push(make_span(from, contents.len()));
                }
            }
        }

        // Log the spans that will be parsed
        if log::log_enabled!(log::Level::Trace) {
            let contents = spans
                .iter()
                .map(|span| String::from_utf8_lossy(working_set.get_span_contents(*span)))
                .collect::<Vec<_>>();

            trace!("parsing: external string, parts: {contents:?}")
        }

        // Check if the whole thing is quoted. If not, it should be a glob
        let quoted =
            (contents.len() >= 3 && contents.starts_with(b"$\"") && contents.ends_with(b"\""))
                || is_quoted(contents);

        // Parse each as its own string
        let exprs: Vec<Expression> = spans
            .into_iter()
            .map(|span| crate::parser::parse_string(working_set, span))
            .collect();

        if exprs
            .iter()
            .all(|expr| matches!(expr.expr, Expr::String(..)))
        {
            // If the exprs are all strings anyway, just collapse into a single string.
            let string = exprs
                .into_iter()
                .map(|expr| {
                    let Expr::String(contents) = expr.expr else {
                        unreachable!("already checked that this was a String")
                    };
                    contents
                })
                .collect::<String>();
            if quoted {
                Expression::new(working_set, Expr::String(string), span, Type::String)
            } else {
                Expression::new(
                    working_set,
                    Expr::GlobPattern(string, false),
                    span,
                    Type::Glob,
                )
            }
        } else {
            // Flatten any string interpolations contained with the exprs.
            let exprs = exprs
                .into_iter()
                .flat_map(|expr| match expr.expr {
                    Expr::StringInterpolation(subexprs) => subexprs,
                    _ => vec![expr],
                })
                .collect();
            // Make an interpolation out of the expressions. Use `GlobInterpolation` if it's a bare
            // word, so that the unquoted state can get passed through to `run-external`.
            if quoted {
                Expression::new(
                    working_set,
                    Expr::StringInterpolation(exprs),
                    span,
                    Type::String,
                )
            } else {
                Expression::new(
                    working_set,
                    Expr::GlobInterpolation(exprs, false),
                    span,
                    Type::Glob,
                )
            }
        }
    } else {
        crate::parser::parse_glob_pattern(working_set, span)
    }
}

fn is_quoted(bytes: &[u8]) -> bool {
    matches!(bytes, [b'\'', .., b'\''] | [b'"', .., b'"'])
}

fn parse_external_arg(working_set: &mut StateWorkingSet, span: Span) -> ExternalArgument {
    let contents = working_set.get_span_contents(span);

    if contents.len() > 3
        && contents.starts_with(b"...")
        && (contents[3] == b'$' || contents[3] == b'[' || contents[3] == b'(')
    {
        ExternalArgument::Spread(crate::parser::parse_value(
            working_set,
            Span::new(span.start + 3, span.end),
            &SyntaxShape::List(Box::new(SyntaxShape::Any)),
        ))
    } else {
        ExternalArgument::Regular(parse_regular_external_arg(working_set, span))
    }
}

pub(crate) fn parse_regular_external_arg(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> Expression {
    match working_set.get_span_contents(span) {
        [b'$', ..] => crate::parser::parse_dollar_expr(working_set, span, &SyntaxShape::Any),
        [b'(', ..] => crate::parser::parse_paren_expr(working_set, span, &SyntaxShape::Any),
        [b'[', ..] => crate::parser::parse_list_expression(working_set, span, &SyntaxShape::Any),
        _ => parse_external_string(working_set, span),
    }
}

pub fn parse_external_call(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    call_span: Span,
) -> Expression {
    trace!("parse external");

    let head_span = spans[0];

    let head_contents = working_set.get_span_contents(head_span);

    let head = if let [b'$' | b'(', ..] = head_contents {
        // the expression is inside external_call, so it's a subexpression
        let arg = crate::parser::parse_expression(working_set, &[head_span]);
        Box::new(arg)
    } else {
        Box::new(parse_external_string(working_set, head_span))
    };

    let args = spans[1..]
        .iter()
        .map(|&span| parse_external_arg(working_set, span))
        .collect();

    Expression::new(
        working_set,
        Expr::ExternalCall(head, args),
        call_span,
        Type::Any,
    )
}

fn ensure_flag_arg_type(
    working_set: &mut StateWorkingSet,
    arg_name: String,
    arg: Expression,
    arg_shape: &SyntaxShape,
    long_name_span: Span,
) -> (Spanned<String>, Expression) {
    if !type_compatible(&arg_shape.to_type(), &arg.ty) {
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
            Expression::garbage(working_set, arg.span),
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

/// Result of attempting to parse a long flag.
///
/// This tri-state enum indicates whether a long flag was found, no flag was found,
/// or the end-of-options delimiter `--` was found (which stops all flag parsing).
enum LongFlagParseResult {
    /// A long flag was successfully parsed: (flag_name, value_expression)
    FoundFlag(Spanned<String>, Option<Expression>),
    /// No long flag found at this position
    NoFlag,
    /// End-of-options delimiter `--` found; stop flag parsing
    EndOfOptions,
}

fn parse_long_flag(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    spans_idx: &mut usize,
    sig: &Signature,
) -> LongFlagParseResult {
    let arg_span = spans[*spans_idx];
    let arg_contents = working_set.get_span_contents(arg_span);

    if arg_contents.starts_with(b"--") {
        // Check for end-of-options delimiter: exactly "--"
        if arg_contents == b"--" {
            return LongFlagParseResult::EndOfOptions;
        }

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

                        let arg = crate::parser::parse_value(working_set, span, arg_shape);
                        let (arg_name, val_expression) = ensure_flag_arg_type(
                            working_set,
                            long_name,
                            arg,
                            arg_shape,
                            Span::new(arg_span.start, arg_span.start + long_name_len + 2),
                        );
                        LongFlagParseResult::FoundFlag(arg_name, Some(val_expression))
                    } else if let Some(arg) = spans.get(*spans_idx + 1) {
                        let arg = crate::parser::parse_value(working_set, *arg, arg_shape);

                        *spans_idx += 1;
                        let (arg_name, val_expression) =
                            ensure_flag_arg_type(working_set, long_name, arg, arg_shape, arg_span);
                        LongFlagParseResult::FoundFlag(arg_name, Some(val_expression))
                    } else {
                        working_set.error(ParseError::MissingFlagParam(
                            arg_shape.to_string(),
                            arg_span,
                        ));
                        // NOTE: still need to cover this incomplete flag in the final expression
                        // see https://github.com/nushell/nushell/issues/16375
                        LongFlagParseResult::FoundFlag(
                            Spanned {
                                item: long_name,
                                span: arg_span,
                            },
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

                        let arg =
                            crate::parser::parse_value(working_set, span, &SyntaxShape::Boolean);

                        let (arg_name, val_expression) = ensure_flag_arg_type(
                            working_set,
                            long_name,
                            arg,
                            &SyntaxShape::Boolean,
                            Span::new(arg_span.start, arg_span.start + long_name_len + 2),
                        );
                        LongFlagParseResult::FoundFlag(arg_name, Some(val_expression))
                    } else {
                        LongFlagParseResult::FoundFlag(
                            Spanned {
                                item: long_name,
                                span: arg_span,
                            },
                            None,
                        )
                    }
                }
            } else {
                let suggestion = did_you_mean(sig.get_names(), &long_name)
                    .map(|name| format!("Did you mean: `--{name}`?"))
                    .unwrap_or("Use `--help` to see available flags".to_owned());
                working_set.error(ParseError::UnknownFlag(
                    sig.name.clone(),
                    long_name.clone(),
                    arg_span,
                    suggestion,
                ));
                LongFlagParseResult::FoundFlag(
                    Spanned {
                        item: long_name.clone(),
                        span: arg_span,
                    },
                    None,
                )
            }
        } else {
            working_set.error(ParseError::NonUtf8(arg_span));
            LongFlagParseResult::FoundFlag(
                Spanned {
                    item: "--".into(),
                    span: arg_span,
                },
                None,
            )
        }
    } else {
        LongFlagParseResult::NoFlag
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
                    "Use `--help` to see available flags".to_owned(),
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
            if positionals_between >= (kw_idx - spans_idx) {
                kw_idx
            } else {
                kw_idx - positionals_between
            }
        } else {
            // Make space for the remaining require positionals, if we can
            // spans_idx < spans.len() is an invariant
            let remaining_spans = spans.len() - (spans_idx + 1);
            // positional_idx can be larger than required_positional.len() if we have optional args
            let remaining_positional = signature
                .required_positional
                .len()
                .saturating_sub(positional_idx + 1);
            // Saturates to 0 when we have too few args
            let extra_spans = remaining_spans.saturating_sub(remaining_positional);
            spans_idx + 1 + extra_spans
        }
    }
}

pub(crate) fn parse_oneof(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    spans_idx: &mut usize,
    possible_shapes: &Vec<SyntaxShape>,
    multispan: bool,
) -> Expression {
    let starting_spans_idx = *spans_idx;
    let mut best_guess = None;
    let mut best_guess_errors = Vec::new();
    let mut max_first_error_offset = 0;
    let mut propagate_error = false;
    for shape in possible_shapes {
        let starting_error_count = working_set.parse_errors.len();
        *spans_idx = starting_spans_idx;
        let value = match multispan {
            true => parse_multispan_value(working_set, spans, spans_idx, shape),
            false => crate::parser::parse_value(working_set, spans[*spans_idx], shape),
        };

        let new_errors = &working_set.parse_errors[starting_error_count..];
        // no new errors found means success
        let Some(first_error_offset) = new_errors.iter().map(|e| e.span().start).min() else {
            return value;
        };

        if first_error_offset > max_first_error_offset {
            // while trying the possible shapes, ignore Expected type errors
            // unless they're inside a block, closure, or expression
            propagate_error = match working_set.parse_errors.last() {
                Some(ParseError::Expected(_, error_span))
                | Some(ParseError::ExpectedWithStringMsg(_, error_span)) => {
                    matches!(
                        shape,
                        SyntaxShape::Block | SyntaxShape::Closure(_) | SyntaxShape::Expression
                    ) && *error_span != spans[*spans_idx]
                }
                _ => true,
            };
            max_first_error_offset = first_error_offset;
            best_guess = Some(value);
            best_guess_errors.clear();
            best_guess_errors.extend_from_slice(new_errors);
        }
        working_set.parse_errors.truncate(starting_error_count);
    }

    // if best_guess results in new errors further than current span, then accept it
    // or propagate_error is marked as true for it
    if max_first_error_offset > spans[starting_spans_idx].start || propagate_error {
        working_set.parse_errors.extend(best_guess_errors);
        best_guess.expect("best_guess should not be None here!")
    } else {
        working_set.error(ParseError::ExpectedWithStringMsg(
            format!("one of a list of accepted shapes: {possible_shapes:?}"),
            spans[starting_spans_idx],
        ));
        Expression::garbage(working_set, spans[starting_spans_idx])
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

            crate::parser::parse_var_with_opt_type(working_set, spans, spans_idx, false).0
        }
        SyntaxShape::RowCondition => {
            trace!("parsing: row condition");
            let arg = crate::parser::parse_row_condition(working_set, &spans[*spans_idx..]);
            *spans_idx = spans.len() - 1;

            arg
        }
        SyntaxShape::MathExpression => {
            trace!("parsing: math expression");

            let arg = crate::parser::parse_math_expression(working_set, &spans[*spans_idx..], None);
            *spans_idx = spans.len() - 1;

            arg
        }
        SyntaxShape::OneOf(possible_shapes) => {
            parse_oneof(working_set, spans, spans_idx, possible_shapes, true)
        }

        SyntaxShape::Expression => {
            trace!("parsing: expression");

            // is it subexpression?
            // Not sure, but let's make it not, so the behavior is the same as previous version of nushell.
            let arg = crate::parser::parse_expression(working_set, &spans[*spans_idx..]);
            *spans_idx = spans.len().saturating_sub(1);

            arg
        }
        SyntaxShape::Signature => {
            trace!("parsing: signature");

            let sig = crate::parser::parse_full_signature(working_set, &spans[*spans_idx..], false);
            *spans_idx = spans.len().saturating_sub(1);

            sig
        }
        SyntaxShape::ExternalSignature => {
            trace!("parsing: external signature");

            let sig = crate::parser::parse_full_signature(working_set, &spans[*spans_idx..], true);
            *spans_idx = spans.len().saturating_sub(1);

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
                    expr: Expression::garbage(working_set, arg_span),
                };
                return Expression::new(
                    working_set,
                    Expr::Keyword(Box::new(keyword)),
                    arg_span,
                    Type::Any,
                );
            }

            let keyword = Keyword {
                keyword: keyword.as_slice().into(),
                span: spans[*spans_idx - 1],
                expr: parse_multispan_value(working_set, spans, spans_idx, arg),
            };

            Expression::new(
                working_set,
                Expr::Keyword(Box::new(keyword.clone())),
                keyword.span.merge(keyword.expr.span),
                keyword.expr.ty,
            )
        }
        _ => {
            // All other cases are single-span values
            let arg_span = spans[*spans_idx];

            crate::parser::parse_value(working_set, arg_span, shape)
        }
    }
}

pub struct ParsedInternalCall {
    pub call: Box<Call>,
    pub output: Type,
    pub call_kind: CallKind,
}

/// Sometimes the arguments of an internal command need to be parsed in dedicated functions, e.g. `parse_module`.
/// If so, `parse_internal_call` should be called with the appropriate parsing level to avoid repetition.
///
/// Defaults to `ArgumentParsingLevel::Full`
#[derive(Default)]
pub enum ArgumentParsingLevel {
    #[default]
    Full,
    /// Parse only the first `k` arguments
    FirstK { k: usize },
}

pub fn parse_internal_call(
    working_set: &mut StateWorkingSet,
    command_span: Span,
    spans: &[Span],
    decl_id: DeclId,
    arg_parsing_level: ArgumentParsingLevel,
) -> ParsedInternalCall {
    trace!("parsing: internal call (decl id: {})", decl_id.get());

    let mut call = Call::new(command_span);
    call.decl_id = decl_id;
    call.head = command_span;
    let _ = working_set.add_span(call.head);

    let decl = working_set.get_decl(decl_id);
    let signature = working_set.get_signature(decl);
    let output = signature.get_output_type();

    let deprecation = decl.deprecation_info();

    // storing the var ID for later due to borrowing issues
    let lib_dirs_var_id = match decl.name() {
        "use" | "overlay use" | "source-env" if decl.is_keyword() => {
            find_dirs_var(working_set, crate::parse_source::LIB_DIRS_VAR)
        }
        "nu-check" if decl.is_builtin() => {
            find_dirs_var(working_set, crate::parse_source::LIB_DIRS_VAR)
        }
        _ => None,
    };

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
            positional_idx = call.positional_iter().count();
        } else {
            working_set.error(ParseError::UnknownState(
                "Alias does not point to internal call.".to_string(),
                command_span,
            ));
            return ParsedInternalCall {
                call: Box::new(call),
                output: Type::Any,
                call_kind: CallKind::Invalid,
            };
        }
    }

    if let Some(var_id) = lib_dirs_var_id {
        call.set_parser_info(
            DIR_VAR_PARSER_INFO.to_owned(),
            Expression::new(working_set, Expr::Var(var_id), call.head, Type::Any),
        );
    }

    if signature.creates_scope {
        working_set.enter_scope();
    }

    let mut end_of_options = false;

    while spans_idx < spans.len() {
        let arg_span = spans[spans_idx];

        let starting_error_count = working_set.parse_errors.len();

        // If we've seen --, skip all flag parsing and go straight to positional parsing
        if !end_of_options {
            // Check if we're on a long flag, if so, parse
            let flag_parse_result = parse_long_flag(working_set, spans, &mut spans_idx, &signature);

            match flag_parse_result {
                LongFlagParseResult::EndOfOptions => {
                    // Treat `--` as an end-of-options marker for all commands,
                    // including wrapped commands. The delimiter itself is
                    // consumed and not forwarded as an argument.
                    end_of_options = true;
                    spans_idx += 1;
                    continue;
                }
                LongFlagParseResult::FoundFlag(long_name, arg) => {
                    // We found a long flag, like --bar
                    if working_set.parse_errors[starting_error_count..]
                        .iter()
                        .any(|x| matches!(x, ParseError::UnknownFlag(_, _, _, _)))
                        && signature.allows_unknown_args
                    {
                        working_set.parse_errors.truncate(starting_error_count);
                        let arg = parse_unknown_arg(working_set, arg_span, &signature);

                        call.add_unknown(arg);
                    } else {
                        call.add_named((long_name, None, arg));
                    }

                    spans_idx += 1;
                    continue;
                }
                LongFlagParseResult::NoFlag => {
                    // No long flag found, continue to short flag parsing
                }
            }
        }

        // Only try short flag parsing if we haven't seen -- yet
        if !end_of_options {
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
                        completion: None,
                    })
                }

                if working_set.parse_errors[starting_error_count..]
                    .iter()
                    .any(|x| matches!(x, ParseError::UnknownFlag(_, _, _, _)))
                    && signature.allows_unknown_args
                {
                    working_set.parse_errors.truncate(starting_error_count);
                    let arg = parse_unknown_arg(working_set, arg_span, &signature);

                    call.add_unknown(arg);
                } else {
                    for flag in short_flags {
                        let _ = working_set.add_span(spans[spans_idx]);

                        if let Some(arg_shape) = flag.arg {
                            if let Some(arg) = spans.get(spans_idx + 1) {
                                let arg = crate::parser::parse_value(working_set, *arg, &arg_shape);
                                let (arg_name, val_expression) = ensure_flag_arg_type(
                                    working_set,
                                    flag.long.clone(),
                                    arg.clone(),
                                    &arg_shape,
                                    spans[spans_idx],
                                );

                                if flag.long.is_empty() {
                                    if let Some(short) = flag.short {
                                        call.add_named((
                                            arg_name,
                                            Some(Spanned {
                                                item: short.to_string(),
                                                span: spans[spans_idx],
                                            }),
                                            Some(val_expression),
                                        ));
                                    }
                                } else {
                                    call.add_named((arg_name, None, Some(val_expression)));
                                }
                                spans_idx += 1;
                            } else {
                                working_set.error(ParseError::MissingFlagParam(
                                    arg_shape.to_string(),
                                    arg_span,
                                ));
                                // NOTE: still need to cover this incomplete flag in the final expression
                                // see https://github.com/nushell/nushell/issues/16375
                                call.add_named((
                                    Spanned {
                                        item: String::new(),
                                        span: spans[spans_idx],
                                    },
                                    None,
                                    None,
                                ));
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
        } // end if !end_of_options (short flags)

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
                    call.add_positional(Expression::garbage(working_set, arg_span));
                } else if positional_idx < signature.required_positional.len() {
                    working_set.error(ParseError::MissingPositional(
                        signature.required_positional[positional_idx].name.clone(),
                        Span::new(spans[spans_idx].start, spans[spans_idx].start),
                        signature.call_signature(),
                    ));
                    call.add_positional(Expression::garbage(working_set, arg_span));
                } else {
                    let rest_shape = match &signature.rest_positional {
                        Some(arg) if matches!(arg.shape, SyntaxShape::ExternalArgument) => {
                            // External args aren't parsed inside lists in spread position.
                            SyntaxShape::Any
                        }
                        Some(arg) => arg.shape.clone(),
                        None => SyntaxShape::Any,
                    };
                    // Parse list of arguments to be spread
                    let args = crate::parser::parse_value(
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

            // Missing arguments before next keyword
            if end == spans_idx {
                let prev_span = if spans_idx == 0 {
                    command_span
                } else {
                    spans[spans_idx - 1]
                };
                let whitespace_span = Span::new(prev_span.end, spans[spans_idx].start);
                working_set.error(ParseError::MissingPositional(
                    positional.name.clone(),
                    whitespace_span,
                    signature.call_signature(),
                ));
                call.add_positional(Expression::garbage(working_set, whitespace_span));
                positional_idx += 1;
                continue;
            }
            debug_assert!(end <= spans.len());

            if spans[..end].is_empty() || spans_idx == end {
                working_set.error(ParseError::MissingPositional(
                    positional.name.clone(),
                    Span::new(spans[spans_idx].end, spans[spans_idx].end),
                    signature.call_signature(),
                ));
                positional_idx += 1;
                continue;
            }

            let compile_error_count = working_set.compile_errors.len();

            // HACK: avoid repeated parsing of argument values in special cases
            // see https://github.com/nushell/nushell/issues/16398
            let arg = match arg_parsing_level {
                ArgumentParsingLevel::FirstK { k } if k <= positional_idx => {
                    Expression::garbage(working_set, spans[spans_idx])
                }
                _ => parse_multispan_value(
                    working_set,
                    &spans[..end],
                    &mut spans_idx,
                    &positional.shape,
                ),
            };

            // HACK: try-catch's signature defines the catch block as a Closure, even though it's
            // used like a Block. Because closures are compiled eagerly, this ends up making the
            // following code technically invalid:
            // ```nu
            // loop { try { } catch {|e| break } }
            // ```
            // Thus, we discard the compilation error here
            if let SyntaxShape::OneOf(ref shapes) = positional.shape {
                for one_shape in shapes {
                    if let SyntaxShape::Keyword(keyword, ..) = one_shape
                        && keyword == b"catch"
                        && let [nu_protocol::CompileError::NotInALoop { .. }] =
                            &working_set.compile_errors[compile_error_count..]
                    {
                        working_set.compile_errors.truncate(compile_error_count);
                    }
                }
            }

            let arg = if !type_compatible(&positional.shape.to_type(), &arg.ty) {
                working_set.error(ParseError::TypeMismatch(
                    positional.shape.to_type(),
                    arg.ty,
                    arg.span,
                ));
                Expression::garbage(working_set, arg.span)
            } else {
                arg
            };

            call.add_positional(arg);
            positional_idx += 1;
        } else if signature.allows_unknown_args {
            let arg = parse_unknown_arg(working_set, arg_span, &signature);

            call.add_unknown(arg);
        } else {
            call.add_positional(Expression::garbage(working_set, arg_span));
            working_set.error(ParseError::ExtraPositional(
                signature.call_signature(),
                arg_span,
            ))
        }

        spans_idx += 1;
    }

    // TODO: Inline `check_call`,
    // move missing positional checking into the while loop above with two pointers.
    // Maybe more `CallKind::Invalid` if errors found during argument parsing.
    let call_kind = check_call(working_set, command_span, &signature, &call);

    deprecation
        .into_iter()
        .filter_map(|entry| entry.parse_warning(&signature.name, &call))
        .for_each(|warning| {
            // FIXME: if two flags are deprecated and both are used in one command,
            // the second flag's deprecation won't show until the first flag is removed
            // (but it won't be flagged as reported until it is actually reported)
            working_set.warning(warning);
        });

    if signature.creates_scope {
        working_set.exit_scope();
    }

    ParsedInternalCall {
        call: Box::new(call),
        output,
        call_kind,
    }
}

pub fn parse_call(working_set: &mut StateWorkingSet, spans: &[Span], head: Span) -> Expression {
    trace!("parsing: call");
    let call_span = Span::concat(spans);

    if spans.is_empty() {
        working_set.error(ParseError::UnknownState(
            "Encountered command with zero spans".into(),
            call_span,
        ));
        return garbage(working_set, head);
    }

    let call_sigil = match working_set.get_span_contents(spans[0]).first() {
        Some(b'^') => Some(b'^'),
        Some(b'%') => Some(b'%'),
        _ => None,
    };

    let mut adjusted_spans = Vec::new();
    let resolution_spans = match call_sigil {
        Some(b'^') | Some(b'%') => {
            adjusted_spans.reserve(spans.len());
            adjusted_spans.push(Span::new(spans[0].start + 1, spans[0].end));
            adjusted_spans.extend_from_slice(&spans[1..]);
            adjusted_spans.as_slice()
        }
        _ => spans,
    };

    // `^` always forces external command parsing and must bypass declaration
    // resolution, even when an internal command with the same name exists.
    if call_sigil == Some(b'^') {
        trace!("parsing: forced external call");
        return parse_external_call(working_set, resolution_spans, call_span);
    }

    // Check if we have a percent sigil with a dynamic head (variable or expression).
    // Supports two token layouts:
    //   - single token: `%$cmd` or `%($cmd)` — stripping `%` leaves `$cmd` / `($cmd)` in [0]
    //   - two tokens:   `%` and `($cmd)`    — stripping `%` leaves an empty span in [0]; head is [1]
    // If so, defer builtin validation to runtime (the IR compiler will rewrite to `run-internal`).
    if call_sigil == Some(b'%') && !resolution_spans.is_empty() {
        // Locate the actual head span, skipping an empty leading span.
        let (head_idx, head_span) = {
            let first = working_set.get_span_contents(resolution_spans[0]);
            if first.is_empty() && resolution_spans.len() > 1 {
                (1, resolution_spans[1])
            } else {
                (0, resolution_spans[0])
            }
        };

        let dynamic_head_contents = working_set.get_span_contents(head_span);
        let is_dynamic_head = !dynamic_head_contents.is_empty()
            && (dynamic_head_contents[0] == b'$' || dynamic_head_contents[0] == b'(');

        if is_dynamic_head {
            trace!("parsing: dynamic percent builtin dispatch");

            let head_expr = crate::parser::parse_expression(working_set, &[head_span]);

            // Create a placeholder call; the IR compiler will rewrite this to `run-internal`.
            let mut call = Call::new(call_span);
            call.decl_id = DeclId::new(0);

            // Store the head expression for the IR compiler to pick up.
            call.set_parser_info(PERCENT_FORCED_BUILTIN_PARSER_INFO.to_string(), head_expr);

            // Mirror the dynamic external-call path by preserving `...expr` as an explicit spread
            // argument so runtime dispatch can forward it without flattening first.
            for arg_span in resolution_spans.iter().skip(head_idx + 1) {
                let contents = working_set.get_span_contents(*arg_span);
                if contents.len() > 3
                    && contents.starts_with(b"...")
                    && (contents[3] == b'$' || contents[3] == b'[' || contents[3] == b'(')
                {
                    let spread_expr = crate::parser::parse_value(
                        working_set,
                        Span::new(arg_span.start + 3, arg_span.end),
                        &SyntaxShape::List(Box::new(SyntaxShape::Any)),
                    );
                    call.arguments.push(Argument::Spread(spread_expr));
                } else {
                    let arg_expr =
                        crate::parser::parse_value(working_set, *arg_span, &SyntaxShape::Any);
                    call.arguments.push(Argument::Positional(arg_expr));
                }
            }

            return Expression::new(
                working_set,
                Expr::Call(Box::new(call)),
                call_span,
                Type::Any,
            );
        }
    }

    let (cmd_start, pos, _name, maybe_decl_id) = if call_sigil == Some(b'%') {
        find_longest_decl_with_command_type(working_set, resolution_spans, CommandType::Builtin)
    } else {
        find_longest_decl(working_set, resolution_spans)
    };

    if let Some(decl_id) = maybe_decl_id {
        // Before the internal parsing we check if there is no let or alias declarations
        // that are missing their name, e.g.: let = 1 or alias = 2
        if resolution_spans.len() > 1 {
            let test_equal = working_set.get_span_contents(resolution_spans[1]);

            if test_equal == [b'='] {
                trace!("incomplete statement");

                working_set.error(ParseError::UnknownState(
                    "Incomplete statement".into(),
                    call_span,
                ));
                return garbage(working_set, call_span);
            }
        }

        let decl = working_set.get_decl(decl_id);

        let parsed_call = if let Some(alias) = decl.as_alias() {
            if let Expression {
                expr: Expr::ExternalCall(head, args),
                span: _,
                span_id: _,
                ty,
            } = &alias.clone().wrapped_call
            {
                trace!("parsing: alias of external call");

                let mut head = head.clone();
                head.span = Span::concat(&resolution_spans[cmd_start..pos]); // replacing the spans preserves syntax highlighting

                let mut final_args = args.clone().into_vec();
                for arg_span in &resolution_spans[pos..] {
                    let arg = parse_external_arg(working_set, *arg_span);
                    final_args.push(arg);
                }

                let expression = Expression::new(
                    working_set,
                    Expr::ExternalCall(head, final_args.into()),
                    Span::concat(spans),
                    ty.clone(),
                );

                return expression;
            } else {
                trace!("parsing: alias of internal call");
                parse_internal_call(
                    working_set,
                    Span::concat(&resolution_spans[cmd_start..pos]),
                    &resolution_spans[pos..],
                    decl_id,
                    ArgumentParsingLevel::Full,
                )
            }
        } else {
            trace!("parsing: internal call");
            parse_internal_call(
                working_set,
                Span::concat(&resolution_spans[cmd_start..pos]),
                &resolution_spans[pos..],
                decl_id,
                ArgumentParsingLevel::Full,
            )
        };

        Expression::new(
            working_set,
            Expr::Call(parsed_call.call),
            call_span,
            parsed_call.output,
        )
    } else {
        if call_sigil == Some(b'%') {
            working_set.error(ParseError::LabeledErrorWithHelp {
                error: "percent sigil requires a built-in command".into(),
                label: "unknown built-in command".into(),
                help:
                    "remove `%` to use normal resolution, or use `^` to run an external command explicitly".into(),
                span: resolution_spans[0],
            });

            // Preserve expression shape for features like completion while retaining the parse error.
            return parse_external_call(working_set, spans, call_span);
        }

        // We might be parsing left-unbounded range ("..10")
        let bytes = working_set.get_span_contents(spans[0]);
        trace!("parsing: range {bytes:?}");
        if let (Some(b'.'), Some(b'.')) = (bytes.first(), bytes.get(1)) {
            trace!("-- found leading range indicator");
            let starting_error_count = working_set.parse_errors.len();

            if let Some(range_expr) = crate::parser::parse_range(working_set, spans[0]) {
                trace!("-- successfully parsed range");
                return range_expr;
            }
            working_set.parse_errors.truncate(starting_error_count);
        }
        trace!("parsing: external call");

        // Otherwise, try external command
        parse_external_call(working_set, spans, call_span)
    }
}

fn find_decl_with_command_type(
    working_set: &StateWorkingSet<'_>,
    name: &[u8],
    command_type: CommandType,
) -> Option<DeclId> {
    // Search all known declarations so `%cmd` can still resolve a built-in even when
    // a custom command with the same name shadows it in normal visibility lookup.
    for idx in (0..working_set.num_decls()).rev() {
        let decl_id = DeclId::new(idx);
        let decl = working_set.get_decl(decl_id);
        if decl.command_type() == command_type && decl.name().as_bytes() == name {
            return Some(decl_id);
        }
    }

    None
}

fn command_name_from_spans(
    working_set: &StateWorkingSet<'_>,
    spans: &[Span],
    prefix: &[u8],
) -> Vec<u8> {
    let mut name = Vec::with_capacity(prefix.len() + spans.len() * 2);
    name.extend(prefix);

    for span in spans {
        let name_part = working_set.get_span_contents(*span);
        if name.is_empty() {
            name.extend(name_part);
        } else {
            name.push(b' ');
            name.extend(name_part);
        }
    }

    name
}

fn find_longest_decl_with_command_type(
    working_set: &StateWorkingSet<'_>,
    spans: &[Span],
    command_type: CommandType,
) -> (
    usize,
    usize,
    Vec<u8>,
    Option<nu_protocol::Id<nu_protocol::marker::Decl>>,
) {
    let mut pos = spans.len();
    let cmd_start = 0;
    let mut name_spans = spans.to_vec();

    let mut name = command_name_from_spans(working_set, &name_spans, b"");

    let mut maybe_decl_id = find_decl_with_command_type(working_set, &name, command_type);

    while maybe_decl_id.is_none() {
        if name_spans.len() <= 1 {
            break;
        }

        name_spans.pop();
        pos -= 1;

        name = command_name_from_spans(working_set, &name_spans, b"");

        maybe_decl_id = find_decl_with_command_type(working_set, &name, command_type);
    }

    (cmd_start, pos, name, maybe_decl_id)
}

pub fn find_longest_decl(
    working_set: &mut StateWorkingSet<'_>,
    spans: &[Span],
) -> (
    usize,
    usize,
    Vec<u8>,
    Option<nu_protocol::Id<nu_protocol::marker::Decl>>,
) {
    find_longest_decl_with_prefix(working_set, spans, b"")
}

pub fn find_longest_decl_with_prefix(
    working_set: &mut StateWorkingSet<'_>,
    spans: &[Span],
    prefix: &[u8],
) -> (
    usize,
    usize,
    Vec<u8>,
    Option<nu_protocol::Id<nu_protocol::marker::Decl>>,
) {
    let mut pos = 0;
    let cmd_start = pos;
    let mut name_spans = vec![];

    for word_span in spans[cmd_start..].iter() {
        // Find the longest group of words that could form a command

        name_spans.push(*word_span);

        pos += 1;
    }

    let mut name = command_name_from_spans(working_set, &name_spans, prefix);

    let mut maybe_decl_id = working_set.find_decl(&name);

    while maybe_decl_id.is_none() {
        // Find the longest command match
        if name_spans.len() <= 1 {
            // Keep the first word even if it does not match -- could be external command
            break;
        }

        name_spans.pop();
        pos -= 1;

        name = command_name_from_spans(working_set, &name_spans, prefix);
        maybe_decl_id = working_set.find_decl(&name);
    }

    // If there is a declaration and there are remaining spans, check if it's an alias.
    // If it is, try to see if there are sub commands
    if let Some(decl_id) = maybe_decl_id
        && pos < spans.len()
    {
        let decl = working_set.get_decl(decl_id);
        if let Some(alias) = decl.as_alias() {
            // Extract the command name from the alias
            // The wrapped_call should be a Call expression for internal commands
            if let Expression {
                expr: Expr::Call(call),
                ..
            } = &alias.wrapped_call
            {
                let aliased_decl_id = call.decl_id;
                let aliased_name = working_set.get_decl(aliased_decl_id).name().to_string();

                // Try to find a longer match using the aliased command name with remaining spans
                let (_, new_pos, new_name, new_decl_id) = find_longest_decl_with_prefix(
                    working_set,
                    &spans[pos..],
                    aliased_name.as_bytes(),
                );

                // If we find a sub command, use it instead.
                if new_decl_id.is_some() && new_pos > 0 {
                    let total_pos = pos + new_pos;
                    return (cmd_start, total_pos, new_name, new_decl_id);
                }
            }
        }
    }

    (cmd_start, pos, name, maybe_decl_id)
}

pub fn parse_attribute(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
) -> (Attribute, Option<String>) {
    let _ = lite_command
        .parts
        .first()
        .filter(|s| working_set.get_span_contents(**s).starts_with(b"@"))
        .expect("Attributes always start with an `@`");

    assert!(
        lite_command.attribute_idx.is_empty(),
        "attributes can't have attributes"
    );

    let mut spans = lite_command.parts.clone();
    if let Some(first) = spans.first_mut() {
        first.start += 1;
    }
    let spans = spans.as_slice();
    let attr_span = Span::concat(spans);

    let (cmd_start, cmd_end, mut name, decl_id) =
        find_longest_decl_with_prefix(working_set, spans, b"attr");

    debug_assert!(name.starts_with(b"attr "));
    let _ = name.drain(..(b"attr ".len()));

    let name_span = Span::concat(&spans[cmd_start..cmd_end]);

    let Ok(name) = String::from_utf8(name) else {
        working_set.error(ParseError::NonUtf8(name_span));
        return (
            Attribute {
                expr: garbage(working_set, attr_span),
            },
            None,
        );
    };

    let Some(decl_id) = decl_id else {
        working_set.error(ParseError::UnknownCommand(name_span));
        return (
            Attribute {
                expr: garbage(working_set, attr_span),
            },
            None,
        );
    };

    let decl = working_set.get_decl(decl_id);

    let parsed_call = match decl.as_alias() {
        // TODO: Once `const def` is available, we should either disallow aliases as attributes OR
        // allow them but rather than using the aliases' name, use the name of the aliased command
        Some(alias) => match &alias.clone().wrapped_call {
            Expression {
                expr: Expr::ExternalCall(..),
                ..
            } => {
                let shell_error = ShellError::NotAConstCommand { span: name_span };
                working_set.error(shell_error.wrap(working_set, attr_span));
                return (
                    Attribute {
                        expr: garbage(working_set, Span::concat(spans)),
                    },
                    None,
                );
            }
            _ => {
                trace!("parsing: alias of internal call");
                parse_internal_call(
                    working_set,
                    name_span,
                    &spans[cmd_end..],
                    decl_id,
                    ArgumentParsingLevel::Full,
                )
            }
        },
        None => {
            trace!("parsing: internal call");
            parse_internal_call(
                working_set,
                name_span,
                &spans[cmd_end..],
                decl_id,
                ArgumentParsingLevel::Full,
            )
        }
    };

    (
        Attribute {
            expr: Expression::new(
                working_set,
                Expr::Call(parsed_call.call),
                Span::concat(spans),
                parsed_call.output,
            ),
        },
        Some(name),
    )
}
