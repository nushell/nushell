use crate::{
    is_math_expression_like,
    lite_parser::LiteCommand,
    parse_def::is_valid_command_name,
    parse_helpers::{garbage, garbage_pipeline},
    parse_keywords::reject_parser_keyword_name,
    parse_pipelines::redirecting_builtin_error,
    parser::{
        ArgumentParsingLevel, CallKind, ParsedInternalCall, parse_call, parse_expression,
        parse_internal_call,
    },
};

use nu_protocol::{
    Alias, ParseError, Span,
    ast::{Argument, Call, Expr, Expression, Pipeline},
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

        let Some(alias_name) = alias_name(working_set, alias_name_expr) else {
            return garbage_pipeline(working_set, spans);
        };

        if spans.len() >= split_id + 3 {
            if !alias_name_fits_module(working_set, &alias_name, module_name, spans[split_id]) {
                return alias_pipeline;
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

            finish_alias(
                working_set,
                &alias_call,
                alias_name,
                expr,
                &lite_command.comments,
            );
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

/// The name of an alias, the first positional of its call to `alias`: a string that is a valid
/// command name and no parser keyword. Otherwise reports why and returns `None`.
pub(crate) fn alias_name(
    working_set: &mut StateWorkingSet,
    alias_name_expr: &Expression,
) -> Option<String> {
    let Some(name) = alias_name_expr.as_string() else {
        working_set.error(ParseError::AliasNotValid(alias_name_expr.span));
        return None;
    };
    if !is_valid_command_name(&name) {
        working_set.error(ParseError::AliasNotValid(alias_name_expr.span));
        None
    } else if reject_parser_keyword_name(working_set, &name, "alias", alias_name_expr.span) {
        None
    } else {
        Some(name)
    }
}

/// Whether an alias may be named `alias_name` in the module `module_name`, if any: not after
/// the module, nor `main`. Reports why not at `name_span`.
pub(crate) fn alias_name_fits_module(
    working_set: &mut StateWorkingSet,
    alias_name: &str,
    module_name: Option<&[u8]>,
    name_span: Span,
) -> bool {
    let Some(mod_name) = module_name else {
        return true;
    };
    if alias_name.as_bytes() == mod_name {
        working_set.error(ParseError::NamedAsModule(
            "alias".to_string(),
            alias_name.to_string(),
            "main".to_string(),
            name_span,
        ));
        return false;
    }
    if alias_name == "main" {
        working_set.error(ParseError::ExportMainAliasNotAllowed(name_span));
        return false;
    }
    true
}

/// Declares `alias_name` an alias of `wrapped_call`, the text after `=` parsed as a call (the
/// end of `parse_alias`). A call to a parser keyword that cannot be aliased is an error
/// instead. The description comes from `comments`, the alias's doc comments, or without any
/// from the text after `=` in `alias_call`, the statement's call to `alias`.
pub(crate) fn finish_alias(
    working_set: &mut StateWorkingSet,
    alias_call: &Call,
    alias_name: String,
    wrapped_call: Expression,
    comments: &[Span],
) {
    let command = match &wrapped_call.expr {
        Expr::Call(rhs_call) => {
            let cmd = working_set.get_decl(rhs_call.decl_id);

            if cmd.is_keyword() && !ALIASABLE_PARSER_KEYWORDS.contains(&cmd.name().as_bytes()) {
                working_set.error(ParseError::CantAliasKeyword(
                    ALIASABLE_PARSER_KEYWORDS
                        .iter()
                        .map(|bytes| String::from_utf8_lossy(bytes).to_string())
                        .collect::<Vec<String>>()
                        .join(", "),
                    rhs_call.head,
                ));
                return;
            }

            Some(cmd.clone_box())
        }
        Expr::ExternalCall(..) => None,
        _ => {
            working_set.error(ParseError::InternalError(
                "Parsed call not a call".into(),
                wrapped_call.span,
            ));
            return;
        }
    };

    let (description, extra_description) = match comments.is_empty() {
        false => working_set.build_desc(comments),
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
