#![allow(clippy::byte_char_slices)]

use crate::{
    Token, TokenContents,
    lex::{LexState, is_assignment_operator, lex, lex_n_tokens},
    lite_parser::{LiteCommand, lite_parse},
    parse_helpers::{
        PERCENT_FORCED_BUILTIN_PARSER_INFO, extract_spread_list, extract_spread_record, garbage,
        garbage_pipeline,
    },
    parse_keywords::{
        is_unaliasable_parser_keyword, parse_alias, parse_attribute_block, parse_const, parse_def,
        parse_export_env, parse_export_in_block, parse_extern, parse_for, parse_hide,
        parse_keyword, parse_let, parse_module, parse_mut, parse_overlay_hide, parse_overlay_new,
        parse_overlay_use, parse_run, parse_run_expr, parse_source, parse_use, parse_where,
        parse_where_expr,
    },
    parse_patterns::parse_pattern,
    parse_pipelines::{parse_block, parse_pipeline_element, redirecting_builtin_error},
    parser::{
        compile_block, expand_to_cell_path, parse_binary, parse_brace_expr, parse_call,
        parse_datetime, parse_directory, parse_dollar_expr, parse_duration, parse_filepath,
        parse_filesize, parse_float, parse_full_cell_path, parse_glob_pattern, parse_int,
        parse_multispan_value, parse_number, parse_oneof, parse_paren_expr, parse_range,
        parse_raw_string, parse_regular_external_arg, parse_signature, parse_signature_helper,
        parse_simple_cell_path, parse_string, parse_string_strict,
    },
    type_check::math_result_type,
};
use itertools::Itertools;
use log::trace;
use nu_protocol::{
    CompareTypes, IntoSpanned, ParseError, PositionalArg, Signature, Span, Spanned, SyntaxShape,
    Type, TypeSet, VarId, ast::*, engine::StateWorkingSet,
};
use std::{collections::HashMap, sync::Arc};

pub fn is_math_expression_like(working_set: &mut StateWorkingSet, span: Span) -> bool {
    let bytes = working_set.get_span_contents(span);
    match bytes {
        [] => return false,
        b"true" | b"false" | b"null" | b"not" | b"if" | b"match" => return true,
        [b'r', b'#', ..] => return true,
        [b'(' | b'{' | b'[' | b'$' | b'"' | b'\'' | b'-', ..] => return true,
        _ => {}
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
    // We need an additional negate match to check if the last error was unexpected
    // or more specifically, if it was `ParseError::InvalidBinaryString`.
    // If so, we suppress the error and stop parsing to the next (which is `parse_range()`).
    if working_set.parse_errors.len() == starting_error_count {
        return true;
    } else if !matches!(
        working_set.parse_errors.last(),
        Some(ParseError::Expected(_, _))
    ) {
        working_set.parse_errors.truncate(starting_error_count);
        return true;
    }
    working_set.parse_errors.truncate(starting_error_count);

    let is_range = parse_range(working_set, span).is_some();
    working_set.parse_errors.truncate(starting_error_count);
    is_range
}

fn is_env_variable_name(bytes: &[u8]) -> bool {
    match bytes {
        [first, rest @ ..] if first == &b'_' || first.is_ascii_alphabetic() => {
            rest.iter().all(|&b| b.is_ascii_alphanumeric() || b == b'_')
        }
        _ => false,
    }
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
        working_set.error(ParseError::Unclosed("]", Span::new(end, end)));
    }

    let inner_span = Span::new(start, end);
    let source = working_set.get_span_contents(inner_span);

    let (output, err) = lex(source, inner_span.start, &[b'\n', b'\r', b','], &[], true);
    if let Some(err) = err {
        working_set.error(err)
    }

    let (mut output, err) = lite_parse(&output, working_set);
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
                let (arg, ty) = if let Some(Spanned {
                    span: trimmed_span, ..
                }) = extract_spread_list(curr_tok.into_spanned(curr_span))
                {
                    // Parse the spread operator
                    // Remove "..." before parsing argument to spread operator
                    command.parts[spans_idx] = trimmed_span;
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

                contained_type = match contained_type {
                    Some(ctype) => Some(ctype.union(ty)),
                    None => Some(ty),
                };

                args.push(arg);

                spans_idx += 1;
            }
        }
    }

    Expression::new(
        working_set,
        Expr::List(args),
        span,
        Type::List(Box::new(if let Some(ty) = contained_type {
            ty
        } else {
            Type::Any
        })),
    )
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

pub(crate) fn parse_table_expression(
    working_set: &mut StateWorkingSet,
    span: Span,
    list_element_shape: &SyntaxShape,
) -> Expression {
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
            working_set.error(ParseError::Unclosed("]", Span::new(end, end)));
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
        return parse_list_expression(working_set, span, list_element_shape);
    };
    if !working_set.get_span_contents(first.span).starts_with(b"[")
        || second.contents != TokenContents::Semicolon
        || rest.is_empty()
    {
        return parse_list_expression(working_set, span, list_element_shape);
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

    Expression::new(working_set, Expr::Table(table), span, ty)
}

fn table_type(head: &[Expression], rows: &[Vec<Expression>]) -> (Type, Vec<ParseError>) {
    let mut errors = vec![];
    let mut rows: Vec<_> = rows.iter().map(|row| row.iter()).collect();

    let column_types = std::iter::from_fn(move || {
        let column = rows
            .iter_mut()
            .filter_map(|row| row.next())
            .map(|col| col.ty.clone());
        Some(Type::supertype_of(column).unwrap_or(Type::Any))
    });

    let mk_error = |span| ParseError::LabeledErrorWithHelp {
        error: "Table column name not string".into(),
        label: "must be a string".into(),
        help: "Table column names should be able to be converted into strings".into(),
        span,
    };

    let ty = head
        .iter()
        .zip(column_types)
        .filter_map(|(expr, col_ty)| {
            if !Type::String.is_subtype_of(&expr.ty) {
                errors.push(mk_error(expr.span));
                None
            } else {
                expr.as_string().zip(Some(col_ty))
            }
        })
        .collect();

    (Type::Table(ty), errors)
}

pub fn parse_block_expression(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    trace!("parsing: block expression");

    let bytes = working_set.get_span_contents(span);

    let mut start = span.start;
    let mut end = span.end;
    let mut is_closed = true;

    if bytes.starts_with(b"{") {
        start += 1;
    } else {
        working_set.error(ParseError::Expected("block", span));
        return garbage(working_set, span);
    }
    if bytes.ends_with(b"}") {
        end -= 1;
    } else {
        working_set.error(ParseError::Unclosed("}", Span::new(end, end)));
        is_closed = false;
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

    if is_closed {
        working_set.exit_scope();
    }

    let block_id = working_set.add_block(Arc::new(output));

    Expression::new(working_set, Expr::Block(block_id), span, Type::Block)
}

pub fn parse_match_block_expression(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let bytes = working_set.get_span_contents(span);

    let mut start = span.start;
    let mut end = span.end;
    let mut is_closed = true;

    if bytes.starts_with(b"{") {
        start += 1;
    } else {
        working_set.error(ParseError::Expected("closure", span));
        return garbage(working_set, span);
    }
    if bytes.ends_with(b"}") {
        end -= 1;
    } else {
        working_set.error(ParseError::Unclosed("}", Span::new(end, end)));
        is_closed = false;
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
        }
        // A match guard
        if connector == b"if" {
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
                return garbage(working_set, span);
            };

            let (tokens, found) = if let Some((pos, _)) = output[position..]
                .iter()
                .find_position(|t| working_set.get_span_contents(t.span) == b"=>")
            {
                if position + pos == position {
                    working_set.error(mk_err());
                    return garbage(working_set, span);
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

            pattern.guard = Some(Box::new(guard));
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
        if is_closed {
            working_set.exit_scope();
        }

        output_matches.push((pattern, result));
    }

    Expression::new(
        working_set,
        Expr::MatchBlock(output_matches),
        span,
        Type::Any,
    )
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
    let mut is_closed = true;

    if bytes.starts_with(b"{") {
        start += 1;
    } else {
        working_set.error(ParseError::Expected("closure", span));
        return garbage(working_set, span);
    }
    if bytes.ends_with(b"}") {
        end -= 1;
    } else {
        working_set.error(ParseError::Unclosed("}", Span::new(end, end)));
        is_closed = false;
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
                    amt_to_skip += token.0;
                    break;
                }
            }

            let end_point = if let Some(span) = end_span {
                span.end
            } else {
                working_set.error(ParseError::Unclosed("|", Span::new(end, end)));
                end
            };

            let signature_span = Span::new(start_point, end_point);
            let signature = parse_signature_helper(working_set, signature_span, false);

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
    if let SyntaxShape::Closure(Some(v)) = shape
        && let Some((sig, sig_span)) = &signature
    {
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

    let mut output = parse_block(working_set, &output[amt_to_skip..], span, false, false);

    // NOTE: closures need to be compiled eagerly due to these reasons:
    //  - their `Block`s (which contains their `IrBlock`) are stored in the working_set
    //  - Ir compiler does not have mutable access to the working_set and can't attach `IrBlock`s
    //  to existing `Block`s
    // so they can't be compiled as part of their parent `Block`'s compilation
    //
    // If the compiler used a mechanism similar to the `EngineState`/`StateWorkingSet` divide, we
    // could defer all compilation and apply the generated delta to `StateWorkingSet` afterwards.
    if working_set.parse_errors.is_empty() {
        compile_block(working_set, &mut output);
    }

    if let Some(signature) = signature {
        output.signature = signature.0;
    }

    output.span = Some(span);

    if is_closed {
        working_set.exit_scope();
    }

    let block_id = working_set.add_block(Arc::new(output));

    Expression::new(working_set, Expr::Closure(block_id), span, Type::Closure)
}

pub fn parse_value(
    working_set: &mut StateWorkingSet,
    span: Span,
    shape: &SyntaxShape,
) -> Expression {
    trace!("parsing: value: {shape}");

    let bytes = working_set.get_span_contents(span);

    if bytes.is_empty() {
        working_set.error(ParseError::IncompleteParser(span));
        return garbage(working_set, span);
    }

    if let SyntaxShape::OneOf(possible_shapes) = shape {
        return parse_oneof(working_set, &[span], &mut 0, possible_shapes, false);
    }

    match bytes[0] {
        b'$' => return parse_dollar_expr(working_set, span, shape),
        b'(' => return parse_paren_expr(working_set, span, shape),
        b'{' => return parse_brace_expr(working_set, span, shape),
        b'[' => match shape {
            SyntaxShape::Any
            | SyntaxShape::List(_)
            | SyntaxShape::Table(_)
            | SyntaxShape::Signature
            | SyntaxShape::ExternalSignature
            | SyntaxShape::Filepath
            | SyntaxShape::String
            | SyntaxShape::GlobPattern
            | SyntaxShape::ExternalArgument => {}

            _ => {
                working_set.error(ParseError::ExpectedWithStringMsg(shape.to_string(), span));
                return Expression::garbage(working_set, span);
            }
        },
        b'r' if bytes.len() > 1 && bytes[1] == b'#' => {
            return parse_raw_string(working_set, span);
        }
        _ => {}
    }

    match shape {
        SyntaxShape::Number => parse_number(working_set, span),
        SyntaxShape::Float => parse_float(working_set, span),
        SyntaxShape::Int => parse_int(working_set, span),
        SyntaxShape::Duration => parse_duration(working_set, span),
        SyntaxShape::DateTime => parse_datetime(working_set, span),
        SyntaxShape::Filesize => parse_filesize(working_set, span),
        SyntaxShape::Range => {
            parse_range(working_set, span).unwrap_or_else(|| garbage(working_set, span))
        }
        // Check for reserved keyword values
        SyntaxShape::Nothing | SyntaxShape::Any if bytes == b"null" => {
            Expression::new(working_set, Expr::Nothing, span, Type::Nothing)
        }
        SyntaxShape::Boolean | SyntaxShape::Any if bytes == b"true" => {
            Expression::new(working_set, Expr::Bool(true), span, Type::Bool)
        }
        SyntaxShape::Boolean | SyntaxShape::Any if bytes == b"false" => {
            Expression::new(working_set, Expr::Bool(false), span, Type::Bool)
        }
        SyntaxShape::Filepath
        | SyntaxShape::Directory
        | SyntaxShape::GlobPattern
        // TODO: this serves for backward compatibility.
        // As a consequence, for commands like `def foo [foo: string] {}`,
        // it forbids usage like `foo true`, have to call it explicitly with `foo "true"`.
        // On the other hand, given current `SyntaxShape` based `parse_value`, `foo 10.0` doesn't raise any error.
        // We want to fix this discrepancy in the future.
        | SyntaxShape::String
            if matches!(bytes, b"true" | b"false" | b"null") =>
        {
            working_set.error(ParseError::ExpectedWithStringMsg(shape.to_string(), span));
            garbage(working_set, span)
        }
        SyntaxShape::Filepath => parse_filepath(working_set, span),
        SyntaxShape::Directory => parse_directory(working_set, span),
        SyntaxShape::GlobPattern => parse_glob_pattern(working_set, span),
        SyntaxShape::String => parse_string(working_set, span),
        SyntaxShape::Binary => parse_binary(working_set, span),
        SyntaxShape::Signature if bytes.starts_with(b"[") => parse_signature(working_set, span, false),
        SyntaxShape::ExternalSignature if bytes.starts_with(b"[") => parse_signature(working_set, span, true),
        SyntaxShape::List(elem) if bytes.starts_with(b"[") => {
            parse_table_expression(working_set, span, elem)
        }
        SyntaxShape::Table(_) if bytes.starts_with(b"[") => {
            parse_table_expression(working_set, span, &SyntaxShape::Any)
        }
        SyntaxShape::CellPath => parse_simple_cell_path(working_set, span),

        // Be sure to return ParseError::Expected(..) if invoked for one of these shapes, but lex
        // stream doesn't start with '{'} -- parsing in SyntaxShape::Any arm depends on this error variant.
        SyntaxShape::Block | SyntaxShape::Closure(..) | SyntaxShape::Record(_) => {
            working_set.error(ParseError::Expected("block, closure or record", span));

            Expression::garbage(working_set, span)
        }

        SyntaxShape::ExternalArgument => parse_regular_external_arg(working_set, span),

        SyntaxShape::Any => {
            if bytes.starts_with(b"[") {
                //parse_value(working_set, span, &SyntaxShape::Table)
                parse_full_cell_path(working_set, None, span)
            } else {
                let shapes = [
                    SyntaxShape::Binary,
                    SyntaxShape::Range,
                    SyntaxShape::Filesize,
                    SyntaxShape::Duration,
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
                garbage(working_set, span)
            }
        }
        _ => {
            working_set.error(ParseError::ExpectedWithStringMsg(shape.to_string(), span));
            garbage(working_set, span)
        }
    }
}

pub fn parse_assignment_operator(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let contents = working_set.get_span_contents(span);

    let operator = match contents {
        b"=" => Operator::Assignment(Assignment::Assign),
        b"+=" => Operator::Assignment(Assignment::AddAssign),
        b"-=" => Operator::Assignment(Assignment::SubtractAssign),
        b"*=" => Operator::Assignment(Assignment::MultiplyAssign),
        b"/=" => Operator::Assignment(Assignment::DivideAssign),
        b"++=" => Operator::Assignment(Assignment::ConcatenateAssign),
        _ => {
            working_set.error(ParseError::Expected("assignment operator", span));
            return garbage(working_set, span);
        }
    };

    Expression::new(working_set, Expr::Operator(operator), span, Type::Any)
}

pub fn parse_assignment_expression(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
) -> Expression {
    trace!("parsing: assignment expression");
    let expr_span = Span::concat(spans);

    // Assignment always has the most precedence, and its right-hand side can be a pipeline
    let Some(op_index) = spans
        .iter()
        .position(|span| is_assignment_operator(working_set.get_span_contents(*span)))
    else {
        working_set.error(ParseError::Expected("assignment expression", expr_span));
        return garbage(working_set, expr_span);
    };

    let lhs_spans = &spans[0..op_index];
    let op_span = spans[op_index];
    let rhs_spans = &spans[(op_index + 1)..];

    if lhs_spans.is_empty() {
        working_set.error(ParseError::Expected(
            "left hand side of assignment",
            op_span,
        ));
        return garbage(working_set, expr_span);
    }

    if rhs_spans.is_empty() {
        working_set.error(ParseError::Expected(
            "right hand side of assignment",
            op_span,
        ));
        return garbage(working_set, expr_span);
    }

    // Parse the lhs and operator as usual for a math expression
    let mut lhs = parse_expression(working_set, lhs_spans, None);
    // make sure that lhs is a mutable variable.
    match &lhs.expr {
        Expr::FullCellPath(p) => {
            if let Expr::Var(var_id) = p.head.expr
                && var_id != nu_protocol::ENV_VARIABLE_ID
                && !working_set.get_variable(var_id).mutable
            {
                working_set.error(ParseError::AssignmentRequiresMutableVar(lhs.span))
            }
        }
        _ => working_set.error(ParseError::AssignmentRequiresVar(lhs.span)),
    }

    let mut operator = parse_assignment_operator(working_set, op_span);

    // Re-parse the right-hand side as a subexpression
    let rhs_span = Span::concat(rhs_spans);

    let (rhs_tokens, rhs_error) = lex(
        working_set.get_span_contents(rhs_span),
        rhs_span.start,
        &[],
        &[],
        false,
    );
    working_set.parse_errors.extend(rhs_error);

    trace!("parsing: assignment right-hand side subexpression");
    let rhs_block = parse_block(working_set, &rhs_tokens, rhs_span, false, true);
    let rhs_ty = rhs_block.output_type();

    // TEMP: double-check that if the RHS block starts with an external call, it must start with a
    // caret. This is to mitigate the change in assignment parsing introduced in 0.97.0 which could
    // result in unintentional execution of commands.
    if let Some(Expr::ExternalCall(head, ..)) = rhs_block
        .pipelines
        .first()
        .and_then(|pipeline| pipeline.elements.first())
        .map(|element| &element.expr.expr)
    {
        let contents = working_set.get_span_contents(Span {
            start: head.span.start - 1,
            end: head.span.end,
        });
        if !contents.starts_with(b"^") {
            working_set.parse_errors.push(ParseError::LabeledErrorWithHelp {
                error: "External command calls must be explicit in assignments".into(),
                label: "add a caret (^) before the command name if you intended to run and capture its output".into(),
                help: "the parsing of assignments was changed in 0.97.0, and this would have previously been treated as a string. Alternatively, quote the string with single or double quotes to avoid it being interpreted as a command name. This restriction may be removed in a future release.".into(),
                span: head.span,
            });
        }
    }

    let rhs_block_id = working_set.add_block(Arc::new(rhs_block));
    let mut rhs = Expression::new(
        working_set,
        Expr::Subexpression(rhs_block_id),
        rhs_span,
        rhs_ty,
    );

    let (result_ty, err) = math_result_type(working_set, &mut lhs, &mut operator, &mut rhs);
    if let Some(err) = err {
        working_set.parse_errors.push(err);
    }

    Expression::new(
        working_set,
        Expr::BinaryOp(Box::new(lhs), Box::new(operator), Box::new(rhs)),
        expr_span,
        result_ty,
    )
}

pub fn parse_operator(working_set: &mut StateWorkingSet, span: Span) -> Expression {
    let contents = working_set.get_span_contents(span);

    let operator = match contents {
        b"==" => Operator::Comparison(Comparison::Equal),
        b"!=" => Operator::Comparison(Comparison::NotEqual),
        b"<" => Operator::Comparison(Comparison::LessThan),
        b"<=" => Operator::Comparison(Comparison::LessThanOrEqual),
        b">" => Operator::Comparison(Comparison::GreaterThan),
        b">=" => Operator::Comparison(Comparison::GreaterThanOrEqual),
        b"=~" | b"like" => Operator::Comparison(Comparison::RegexMatch),
        b"!~" | b"not-like" => Operator::Comparison(Comparison::NotRegexMatch),
        b"in" => Operator::Comparison(Comparison::In),
        b"not-in" => Operator::Comparison(Comparison::NotIn),
        b"has" => Operator::Comparison(Comparison::Has),
        b"not-has" => Operator::Comparison(Comparison::NotHas),
        b"starts-with" => Operator::Comparison(Comparison::StartsWith),
        b"not-starts-with" => Operator::Comparison(Comparison::NotStartsWith),
        b"ends-with" => Operator::Comparison(Comparison::EndsWith),
        b"not-ends-with" => Operator::Comparison(Comparison::NotEndsWith),
        b"+" => Operator::Math(Math::Add),
        b"-" => Operator::Math(Math::Subtract),
        b"*" => Operator::Math(Math::Multiply),
        b"/" => Operator::Math(Math::Divide),
        b"//" => Operator::Math(Math::FloorDivide),
        b"mod" => Operator::Math(Math::Modulo),
        b"**" => Operator::Math(Math::Pow),
        b"++" => Operator::Math(Math::Concatenate),
        b"bit-or" => Operator::Bits(Bits::BitOr),
        b"bit-xor" => Operator::Bits(Bits::BitXor),
        b"bit-and" => Operator::Bits(Bits::BitAnd),
        b"bit-shl" => Operator::Bits(Bits::ShiftLeft),
        b"bit-shr" => Operator::Bits(Bits::ShiftRight),
        b"or" => Operator::Boolean(Boolean::Or),
        b"xor" => Operator::Boolean(Boolean::Xor),
        b"and" => Operator::Boolean(Boolean::And),
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
            return garbage(working_set, span);
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
            return garbage(working_set, span);
        }
        b"contains" => {
            working_set.error(ParseError::UnknownOperator(
                "contains",
                "Did you mean 'has'?",
                span,
            ));
            return garbage(working_set, span);
        }
        b"%" => {
            working_set.error(ParseError::UnknownOperator(
                "%",
                "Did you mean 'mod'?",
                span,
            ));
            return garbage(working_set, span);
        }
        b"&" => {
            working_set.error(ParseError::UnknownOperator(
                "&",
                "Did you mean 'bit-and'?",
                span,
            ));
            return garbage(working_set, span);
        }
        b"<<" => {
            working_set.error(ParseError::UnknownOperator(
                "<<",
                "Did you mean 'bit-shl'?",
                span,
            ));
            return garbage(working_set, span);
        }
        b">>" => {
            working_set.error(ParseError::UnknownOperator(
                ">>",
                "Did you mean 'bit-shr'?",
                span,
            ));
            return garbage(working_set, span);
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
            return garbage(working_set, span);
        }
        op if is_assignment_operator(op) => {
            working_set.error(ParseError::Expected("a non-assignment operator", span));
            return garbage(working_set, span);
        }
        _ => {
            working_set.error(ParseError::Expected("operator", span));
            return garbage(working_set, span);
        }
    };

    Expression::new(working_set, Expr::Operator(operator), span, Type::Any)
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
    let mut last_prec = u8::MAX;

    let first_span = working_set.get_span_contents(spans[0]);

    let mut not_start_spans = vec![];

    if first_span == b"if" || first_span == b"match" {
        // If expression
        if spans.len() > 1 {
            return parse_call(working_set, spans, spans[0], None);
        } else {
            working_set.error(ParseError::Expected(
                "expression",
                Span::new(spans[0].end, spans[0].end),
            ));
            return garbage(working_set, spans[0]);
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
            return garbage(working_set, spans[idx - 1]);
        }
    }

    let mut lhs = parse_value(working_set, spans[idx], &SyntaxShape::Any);

    for not_start_span in not_start_spans.iter().rev() {
        lhs = Expression::new(
            working_set,
            Expr::UnaryNot(Box::new(lhs)),
            Span::new(*not_start_span, spans[idx].end),
            Type::Bool,
        );
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

            expr_stack.push(Expression::garbage(working_set, spans[idx - 1]));
            let missing_span = Span::new(spans[idx - 1].end, spans[idx - 1].end);
            expr_stack.push(Expression::garbage(working_set, missing_span));

            break;
        }

        let content = working_set.get_span_contents(spans[idx]);
        // allow `if` to be a special value for assignment.

        if content == b"if" || content == b"match" {
            let rhs = parse_call(working_set, &spans[idx..], spans[0], None);
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
                return garbage(working_set, spans[idx - 1]);
            }
        }
        let mut rhs = parse_value(working_set, spans[idx], &SyntaxShape::Any);

        for not_start_span in not_start_spans.iter().rev() {
            rhs = Expression::new(
                working_set,
                Expr::UnaryNot(Box::new(rhs)),
                Span::new(*not_start_span, spans[idx].end),
                Type::Bool,
            );
        }
        not_start_spans.clear();

        // Parsing power must be right-associative unlike most operations which are left
        // Hence, we should not collapse if the last and current operations are both power
        let is_left_associative =
            op.expr != Expr::Operator(Operator::Math(Math::Pow)) && op_prec <= last_prec;

        while is_left_associative && expr_stack.len() > 1 {
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
            expr_stack.push(Expression::new(
                working_set,
                Expr::BinaryOp(Box::new(lhs), Box::new(op), Box::new(rhs)),
                op_span,
                result_ty,
            ));
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
        expr_stack.push(Expression::new(
            working_set,
            Expr::BinaryOp(Box::new(lhs), Box::new(op), Box::new(rhs)),
            binary_op_span,
            result_ty,
        ));
    }

    expr_stack
        .pop()
        .expect("internal error: expression stack empty")
}

pub fn parse_expression(
    working_set: &mut StateWorkingSet,
    spans: &[Span],
    input_type: Option<Type>,
) -> Expression {
    trace!("parsing: expression");

    let mut pos = 0;
    let mut shorthand = vec![];

    while pos < spans.len() {
        // Check if there is any environment shorthand
        let name = working_set.get_span_contents(spans[pos]);

        let split: Vec<_> = name.splitn(2, |x| *x == b'=').collect();
        if split.len() != 2 || !is_env_variable_name(split[0]) {
            break;
        }

        let point = split[0].len() + 1;
        let starting_error_count = working_set.parse_errors.len();

        let rhs = if spans[pos].start + point < spans[pos].end {
            let rhs_span = Span::new(spans[pos].start + point, spans[pos].end);
            if split[1].starts_with(b"$") {
                parse_dollar_expr(working_set, rhs_span, &SyntaxShape::Any)
            } else {
                parse_string_strict(working_set, rhs_span)
            }
        } else {
            Expression::new(
                working_set,
                Expr::String(String::new()),
                Span::unknown(),
                Type::Nothing,
            )
        };

        let lhs_span = Span::new(spans[pos].start, spans[pos].start + point - 1);
        let lhs = parse_string_strict(working_set, lhs_span);

        if starting_error_count == working_set.parse_errors.len() {
            shorthand.push((lhs, rhs));
            pos += 1;
        } else {
            working_set.parse_errors.truncate(starting_error_count);
            break;
        }
    }

    if pos == spans.len() {
        working_set.error(ParseError::UnknownCommand(spans[0]));
        return garbage(working_set, Span::concat(spans));
    }

    let output = if spans[pos..]
        .iter()
        .any(|span| is_assignment_operator(working_set.get_span_contents(*span)))
    {
        parse_assignment_expression(working_set, &spans[pos..])
    } else if is_math_expression_like(working_set, spans[pos]) {
        parse_math_expression(working_set, &spans[pos..], None)
    } else {
        let bytes = working_set.get_span_contents(spans[pos]).to_vec();

        // For now, check for special parses of certain keywords
        match bytes.as_slice() {
            b"def" | b"extern" | b"for" | b"module" | b"use" | b"source" | b"alias" | b"export"
            | b"export-env" | b"hide" => {
                working_set.error(ParseError::BuiltinCommandInPipeline(
                    String::from_utf8(bytes)
                        .expect("builtin commands bytes should be able to convert to string"),
                    spans[0],
                ));

                parse_call(working_set, &spans[pos..], spans[0], input_type)
            }
            b"const" | b"mut" => {
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
                parse_call(working_set, &spans[pos..], spans[0], input_type)
            }
            b"overlay" => {
                if spans.len() > 1 && working_set.get_span_contents(spans[1]) == b"list" {
                    // whitelist 'overlay list'
                    parse_call(working_set, &spans[pos..], spans[0], input_type)
                } else {
                    working_set.error(ParseError::BuiltinCommandInPipeline(
                        "overlay".into(),
                        spans[0],
                    ));

                    parse_call(working_set, &spans[pos..], spans[0], input_type)
                }
            }
            b"where" => parse_where_expr(working_set, &spans[pos..]),
            b"run" => parse_run_expr(working_set, &spans[pos..]),
            #[cfg(feature = "plugin")]
            b"plugin" => {
                if spans.len() > 1 && working_set.get_span_contents(spans[1]) == b"use" {
                    // only 'plugin use' is banned
                    working_set.error(ParseError::BuiltinCommandInPipeline(
                        "plugin use".into(),
                        spans[0],
                    ));
                }

                parse_call(working_set, &spans[pos..], spans[0], input_type)
            }

            _ => parse_call(working_set, &spans[pos..], spans[0], input_type),
        }
    };

    if !shorthand.is_empty() {
        let with_env = working_set.find_decl(b"with-env");
        if let Some(decl_id) = with_env {
            let mut block = Block::default();
            let ty = output.ty.clone();
            block.pipelines = vec![Pipeline::from_vec(vec![output])];
            block.span = Some(Span::concat(spans));

            compile_block(working_set, &mut block);

            let block_id = working_set.add_block(Arc::new(block));

            let mut env_vars = vec![];
            for sh in shorthand {
                env_vars.push(RecordItem::Pair(sh.0, sh.1));
            }

            let arguments = vec![
                Argument::Positional(Expression::new(
                    working_set,
                    Expr::Record(env_vars),
                    Span::concat(&spans[..pos]),
                    Type::Any,
                )),
                Argument::Positional(Expression::new(
                    working_set,
                    Expr::Closure(block_id),
                    Span::concat(&spans[pos..]),
                    Type::Closure,
                )),
            ];

            let expr = Expr::Call(Box::new(Call {
                head: Span::unknown(),
                decl_id,
                arguments,
                parser_info: HashMap::new(),
            }));

            Expression::new(working_set, expr, Span::concat(spans), ty)
        } else {
            output
        }
    } else {
        output
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
                let call_expr = parse_call(
                    working_set,
                    &lite_command.parts,
                    lite_command.parts[0],
                    None,
                );

                if let Expression {
                    expr: Expr::Call(call),
                    ..
                } = call_expr
                    && !call
                        .parser_info
                        .contains_key(PERCENT_FORCED_BUILTIN_PARSER_INFO)
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
    let name = lite_command
        .command_parts()
        .first()
        .map(|s| working_set.get_span_contents(*s))
        .unwrap_or(b"");

    match name {
        // `parse_def` and `parse_extern` work both with and without attributes
        b"def" => parse_def(working_set, lite_command, None).0,
        b"extern" => parse_extern(working_set, lite_command, None),
        // `parse_export_in_block` also handles attributes by itself
        b"export" => parse_export_in_block(working_set, lite_command),
        b"export-env" => parse_export_env(working_set, &lite_command.parts).0,
        // Other definitions can't have attributes, so we handle attributes here with parse_attribute_block
        _ if lite_command.has_attributes() => parse_attribute_block(working_set, lite_command),
        b"let" => parse_let(
            working_set,
            &lite_command
                .parts_including_redirection()
                .collect::<Vec<Span>>(),
        ),
        b"const" => parse_const(working_set, &lite_command.parts).0,
        b"mut" => parse_mut(
            working_set,
            &lite_command
                .parts_including_redirection()
                .collect::<Vec<Span>>(),
        ),
        b"for" => {
            let expr = parse_for(working_set, lite_command);
            Pipeline::from_vec(vec![expr])
        }
        b"alias" => parse_alias(working_set, lite_command, None),
        b"module" => parse_module(working_set, lite_command, None).0,
        b"use" => parse_use(working_set, lite_command, None).0,
        b"overlay" => {
            if let Some(redirection) = lite_command.redirection.as_ref() {
                working_set.error(redirecting_builtin_error("overlay", redirection));
                return garbage_pipeline(working_set, &lite_command.parts);
            }
            parse_keyword(working_set, lite_command)
        }
        b"source" | b"source-env" => parse_source(working_set, lite_command),
        b"run" => parse_run(working_set, lite_command),
        b"hide" => parse_hide(working_set, lite_command),
        b"where" => parse_where(working_set, lite_command),
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
                return garbage_pipeline(working_set, &lite_command.parts);
            }
            parse_keyword(working_set, lite_command)
        }
        _ => {
            let element = parse_pipeline_element(working_set, lite_command, Type::Any);

            // There is still a chance to make `parse_pipeline_element` parse into
            // some keyword that should apply side effects first, Example:
            //
            // module a { export alias b = overlay use first.nu };
            // use a
            // a b
            //
            // In this case, `a b` will be parsed as a pipeline element, which leads
            // to the `overlay use` command.
            // In this case, we need to ensure that the side effects of these keywords
            // are applied.
            if let Expression {
                expr: Expr::Call(call),
                ..
            } = &element.expr
            {
                // Dynamic percent dispatch stores a placeholder call plus parser
                // metadata for later IR rewrite. Skip parser-keyword side-effects lookup here,
                // because there is no declaration to resolve yet.
                if call
                    .parser_info
                    .contains_key(PERCENT_FORCED_BUILTIN_PARSER_INFO)
                {
                    return Pipeline {
                        elements: vec![element],
                    };
                }

                // Apply parse keyword side effects
                let cmd = working_set.get_decl(call.decl_id);
                match cmd.name() {
                    "overlay hide" => return parse_overlay_hide(working_set, call.clone()),
                    "overlay new" => return parse_overlay_new(working_set, call.clone()),
                    "overlay use" => return parse_overlay_use(working_set, call.clone()),
                    _ => { /* this alias is not a parser keyword */ }
                }
            }
            Pipeline {
                elements: vec![element],
            }
        }
    }
}

fn check_record_key_or_value(
    working_set: &StateWorkingSet,
    expr: &Expression,
    position: &str,
) -> Option<ParseError> {
    let bareword_error = |string_value: &Expression| {
        working_set
            .get_span_contents(string_value.span)
            .iter()
            .find_position(|b| **b == b':')
            .map(|(i, _)| {
                let colon_position = i + string_value.span.start;
                ParseError::InvalidLiteral(
                    "colon".to_string(),
                    format!("bare word specifying record {position}"),
                    Span::new(colon_position, colon_position + 1),
                )
            })
    };
    let value_span = working_set.get_span_contents(expr.span);
    match expr.expr {
        Expr::String(_) => {
            if ![b'"', b'\'', b'`'].contains(&value_span[0]) {
                bareword_error(expr)
            } else {
                None
            }
        }
        Expr::StringInterpolation(ref expressions) => {
            if value_span[0] != b'$' {
                expressions
                    .iter()
                    .filter(|expr| matches!(expr.expr, Expr::String(_)))
                    .filter_map(bareword_error)
                    .next()
            } else {
                None
            }
        }
        _ => None,
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
        return garbage(working_set, span);
    }

    let mut unclosed = false;
    let mut extra_tokens = false;
    if bytes.ends_with(b"}") {
        end -= 1;
    } else {
        unclosed = true;
    }

    let inner_span = Span::new(start, end);

    let mut lex_state = LexState {
        input: working_set.get_span_contents(inner_span),
        output: Vec::new(),
        error: None,
        span_offset: start,
    };
    while !lex_state.input.is_empty() {
        if let Some(ParseError::Unbalanced(left, right, _)) = lex_state.error.as_ref()
            && *left == "{"
            && *right == "}"
        {
            extra_tokens = true;
            unclosed = false;
            break;
        }
        let additional_whitespace = &[b'\n', b'\r', b','];
        if lex_n_tokens(&mut lex_state, additional_whitespace, &[b':'], true, 1) < 1 {
            break;
        };
        let span = lex_state
            .output
            .last()
            .expect("should have gotten 1 token")
            .span;
        let contents = working_set.get_span_contents(span);
        if extract_spread_record(contents.into_spanned(span)).is_some() {
            // This was a spread operator, so there's no value
            continue;
        }
        // Get token for colon
        if lex_n_tokens(&mut lex_state, additional_whitespace, &[b':'], true, 1) < 1 {
            break;
        };
        // Get token for value
        if lex_n_tokens(&mut lex_state, additional_whitespace, &[], true, 1) < 1 {
            break;
        };
    }
    let (tokens, err) = (lex_state.output, lex_state.error);

    if unclosed {
        working_set.error(ParseError::Unclosed("}", Span::new(end, end)));
    } else if extra_tokens {
        working_set.error(ParseError::ExtraTokensAfterClosingDelimiter(Span::new(
            lex_state.span_offset,
            end,
        )));
    }

    if let Some(err) = err {
        working_set.error(err);
    }

    let mut output = vec![];
    let mut idx = 0;

    let mut field_types = Some(vec![]);
    while idx < tokens.len() {
        let curr_span = tokens[idx].span;
        let curr_tok = working_set.get_span_contents(curr_span);
        if let Some(Spanned { span, .. }) = extract_spread_record(curr_tok.into_spanned(curr_span))
        {
            // Parse spread operator
            let inner = parse_value(working_set, span, &SyntaxShape::record());
            idx += 1;

            match &inner.ty {
                Type::Record(inner_fields) => {
                    if let Some(fields) = &mut field_types {
                        for (field, ty) in inner_fields.iter() {
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
            let field_token = &tokens[idx];
            let field = if field_token.contents != TokenContents::Item {
                working_set.error(ParseError::Expected(
                    "item in record key position",
                    Span::new(field_token.span.start, field_token.span.end),
                ));
                garbage(working_set, curr_span)
            } else {
                let field = parse_value(working_set, curr_span, &SyntaxShape::String);
                if let Some(error) = check_record_key_or_value(working_set, &field, "key") {
                    working_set.error(error);
                    garbage(working_set, field.span)
                } else {
                    field
                }
            };

            idx += 1;
            if idx == tokens.len() {
                working_set.error(ParseError::Expected(
                    "':'",
                    Span::new(curr_span.end, curr_span.end),
                ));
                output.push(RecordItem::Pair(
                    garbage(working_set, curr_span),
                    garbage(working_set, Span::new(curr_span.end, curr_span.end)),
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
                    garbage(
                        working_set,
                        Span::new(colon_span.start, tokens[tokens.len() - 1].span.end),
                    ),
                ));
                break;
            }
            if idx == tokens.len() {
                working_set.error(ParseError::Expected(
                    "value for record field",
                    Span::new(colon_span.end, colon_span.end),
                ));
                output.push(RecordItem::Pair(
                    garbage(working_set, Span::new(curr_span.start, colon_span.end)),
                    garbage(
                        working_set,
                        Span::new(colon_span.end, tokens[tokens.len() - 1].span.end),
                    ),
                ));
                break;
            }

            let value_token = &tokens[idx];
            let value = if value_token.contents != TokenContents::Item {
                working_set.error(ParseError::Expected(
                    "item in record value position",
                    Span::new(value_token.span.start, value_token.span.end),
                ));
                garbage(
                    working_set,
                    Span::new(value_token.span.start, value_token.span.end),
                )
            } else {
                let value = parse_value(working_set, tokens[idx].span, &SyntaxShape::Any);
                if let Some(parse_error) = check_record_key_or_value(working_set, &value, "value") {
                    working_set.error(parse_error);
                    garbage(working_set, value.span)
                } else {
                    value
                }
            };
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

    Expression::new(
        working_set,
        Expr::Record(output),
        span,
        if let Some(fields) = field_types {
            Type::Record(fields.into())
        } else {
            Type::Any
        },
    )
}
