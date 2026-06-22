use crate::{
    is_math_expression_like,
    lite_parser::LiteCommand,
    parse_helpers::{garbage, garbage_pipeline},
    parse_pipelines::redirecting_builtin_error,
    parser::{
        ArgumentParsingLevel, CallKind, ParsedInternalCall, parse_call, parse_expression,
        parse_internal_call,
    },
};

use nu_protocol::{
    Alias, ParseError, Span,
    ast::{Argument, Expr, Expression, Pipeline},
    engine::StateWorkingSet,
};

use crate::ALIASABLE_PARSER_KEYWORDS;

fn check_alias_name<'a>(working_set: &mut StateWorkingSet, spans: &'a [Span]) -> Option<&'a Span> {
    let command_len = if !spans.is_empty() {
        if working_set.get_span_contents(spans[0]) == b"export" {
            2
        } else {
            1
        }
    } else {
        return None;
    };

    if spans.len() == command_len {
        None
    } else if spans.len() < command_len + 3 {
        if working_set.get_span_contents(spans[command_len]) == b"=" {
            let name = String::from_utf8_lossy(
                working_set.get_span_contents(Span::concat(&spans[..command_len])),
            );
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
        let name = String::from_utf8_lossy(
            working_set.get_span_contents(Span::concat(&spans[..command_len])),
        );
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

pub fn parse_alias(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
    module_name: Option<&[u8]>,
) -> Pipeline {
    let spans = &lite_command.parts;

    let (name_span, split_id) =
        if spans.len() > 1 && working_set.get_span_contents(spans[0]) == b"export" {
            (spans[1], 2)
        } else {
            (spans[0], 1)
        };

    let name = working_set.get_span_contents(name_span);

    if name != b"alias" {
        working_set.error(ParseError::InternalError(
            "Alias statement unparsable".into(),
            Span::concat(spans),
        ));
        return garbage_pipeline(working_set, spans);
    }
    if let Some(redirection) = lite_command.redirection.as_ref() {
        working_set.error(redirecting_builtin_error("alias", redirection));
        return garbage_pipeline(working_set, spans);
    }

    if let Some(span) = check_alias_name(working_set, spans) {
        return Pipeline::from_vec(vec![garbage(working_set, *span)]);
    }

    if let Some(decl_id) = working_set.find_decl(b"alias") {
        let (command_spans, rest_spans) = spans.split_at(split_id);

        let original_starting_error_count = working_set.parse_errors.len();

        let ParsedInternalCall {
            call: alias_call,
            output,
            call_kind,
        } = parse_internal_call(
            working_set,
            Span::concat(command_spans),
            rest_spans,
            decl_id,
            ArgumentParsingLevel::Full,
            None,
        );

        working_set
            .parse_errors
            .truncate(original_starting_error_count);

        let alias_pipeline = Pipeline::from_vec(vec![Expression::new(
            working_set,
            Expr::Call(alias_call.clone()),
            Span::concat(spans),
            output,
        )]);

        if call_kind == CallKind::Help {
            return alias_pipeline;
        }

        let Some(alias_name_expr) = alias_call.positional_iter().next() else {
            working_set.error(ParseError::UnknownState(
                "Missing positional after call check".to_string(),
                Span::concat(spans),
            ));
            return garbage_pipeline(working_set, spans);
        };

        let alias_name = if let Some(name) = alias_name_expr.as_string() {
            if name.contains('#')
                || name.contains('^')
                || name.contains('%')
                || name.parse::<bytesize::ByteSize>().is_ok()
                || name.parse::<f64>().is_ok()
            {
                working_set.error(ParseError::AliasNotValid(alias_name_expr.span));
                return garbage_pipeline(working_set, spans);
            } else {
                name
            }
        } else {
            working_set.error(ParseError::AliasNotValid(alias_name_expr.span));
            return garbage_pipeline(working_set, spans);
        };

        if spans.len() >= split_id + 3 {
            if let Some(mod_name) = module_name {
                if alias_name.as_bytes() == mod_name {
                    working_set.error(ParseError::NamedAsModule(
                        "alias".to_string(),
                        alias_name,
                        "main".to_string(),
                        spans[split_id],
                    ));

                    return alias_pipeline;
                }

                if alias_name == "main" {
                    working_set.error(ParseError::ExportMainAliasNotAllowed(spans[split_id]));
                    return alias_pipeline;
                }
            }

            let _equals = working_set.get_span_contents(spans[split_id + 1]);

            let replacement_spans = &spans[(split_id + 2)..];
            let first_bytes = working_set.get_span_contents(replacement_spans[0]);

            if first_bytes != b"if"
                && first_bytes != b"match"
                && is_math_expression_like(working_set, replacement_spans[0])
            {
                let starting_error_count = working_set.parse_errors.len();
                let expr = parse_expression(working_set, replacement_spans, None);
                working_set.parse_errors.truncate(starting_error_count);

                working_set.error(ParseError::CantAliasExpression(
                    expr.expr.description().to_string(),
                    replacement_spans[0],
                ));
                return alias_pipeline;
            }

            let starting_error_count = working_set.parse_errors.len();
            working_set.search_predecls = false;

            let expr = parse_call(working_set, replacement_spans, replacement_spans[0], None);

            working_set.search_predecls = true;

            if starting_error_count != working_set.parse_errors.len()
                && let Some(e) = working_set.parse_errors.get(starting_error_count)
            {
                if let ParseError::MissingPositional(..)
                | ParseError::MissingRequiredFlag(..)
                | ParseError::MissingFlagParam(..) = e
                {
                    working_set
                        .parse_errors
                        .truncate(original_starting_error_count);
                } else {
                    return garbage_pipeline(working_set, replacement_spans);
                }
            }

            let (command, wrapped_call) = match expr {
                Expression {
                    expr: Expr::Call(ref rhs_call),
                    ..
                } => {
                    let cmd = working_set.get_decl(rhs_call.decl_id);

                    if cmd.is_keyword()
                        && !ALIASABLE_PARSER_KEYWORDS.contains(&cmd.name().as_bytes())
                    {
                        working_set.error(ParseError::CantAliasKeyword(
                            ALIASABLE_PARSER_KEYWORDS
                                .iter()
                                .map(|bytes| String::from_utf8_lossy(bytes).to_string())
                                .collect::<Vec<String>>()
                                .join(", "),
                            rhs_call.head,
                        ));
                        return alias_pipeline;
                    }

                    (Some(cmd.clone_box()), expr)
                }
                Expression {
                    expr: Expr::ExternalCall(..),
                    ..
                } => (None, expr),
                _ => {
                    working_set.error(ParseError::InternalError(
                        "Parsed call not a call".into(),
                        expr.span,
                    ));
                    return alias_pipeline;
                }
            };

            let (description, extra_description) = match lite_command.comments.is_empty() {
                false => working_set.build_desc(&lite_command.comments),
                true => match alias_call.arguments.get(1) {
                    Some(Argument::Positional(Expression {
                        expr: Expr::Keyword(kw),
                        ..
                    })) => {
                        let aliased = working_set.get_span_contents(kw.expr.span);
                        (
                            format!("Alias for `{}`", String::from_utf8_lossy(aliased)),
                            String::new(),
                        )
                    }
                    _ => ("User declared alias".into(), String::new()),
                },
            };

            let decl = Alias {
                name: alias_name,
                command,
                wrapped_call,
                description,
                extra_description,
            };

            working_set.add_decl(Box::new(decl));
        }

        if spans.len() == 2 && working_set.get_span_contents(spans[1]).contains(&b'=') {
            let arg = String::from_utf8_lossy(working_set.get_span_contents(spans[1]));

            let (name, initial_value) = arg.split_once('=').unwrap_or((&arg, ""));

            let name = if name.is_empty() { "{name}" } else { name };
            let initial_value = if initial_value.is_empty() {
                "{initial_value}"
            } else {
                initial_value
            };

            working_set.error(ParseError::IncorrectValue(
                "alias argument".into(),
                spans[1],
                format!("Make sure to put spaces around '=': alias {name} = {initial_value}"),
            ))
        } else if spans.len() < 4 {
            working_set.error(ParseError::IncorrectValue(
                "Incomplete alias".into(),
                Span::concat(&spans[..split_id]),
                "incomplete alias".into(),
            ));
        }

        return alias_pipeline;
    }

    working_set.error(ParseError::InternalError(
        "Alias statement unparsable".into(),
        Span::concat(spans),
    ));

    garbage_pipeline(working_set, spans)
}
