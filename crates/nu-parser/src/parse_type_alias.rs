use crate::{
    lex::lex_signature,
    lite_parser::LiteCommand,
    parse_helpers::garbage_pipeline,
    parse_keywords::find_keyword_decl,
    parse_pipelines::redirecting_builtin_error,
    parse_shape_specs::{BUILTIN_TYPE_NAMES, parse_shape_name},
    parser::{ArgumentParsingLevel, CallKind, ParsedInternalCall, parse_internal_call},
};
use nu_protocol::{
    ParseError, Span, SyntaxShape, Type, TypeAlias, TypeAliasId,
    ast::{Call, Expr, Expression, Keyword, Pipeline},
    engine::StateWorkingSet,
};

/// Whether `name` can be used as a type alias name: a letter or `_`, then letters, digits, `_`
/// and `-`. Other characters would be split off wherever the name is used: `@` starts a
/// completer, `:` and `,` separate record fields, and `<` and `>` hold type parameters.
fn is_valid_type_alias_name(name: &[u8]) -> bool {
    let Ok(name) = std::str::from_utf8(name) else {
        return false;
    };
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && chars.all(|c| c.is_alphabetic() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Parse `[export] type <name> = <type>` and define the type alias in the current scope.
///
/// The right-hand side is any type a parameter annotation accepts, without a `@completer`: a
/// completer belongs to a parameter, so it is written there, as in `c: Color@[red green blue]`.
/// A right-hand side that doesn't parse still defines the name, as `any`, so each mistake is
/// reported once, here, and not again at every use.
///
/// Returns the call to the `type` keyword and, unless the name was rejected, the new alias.
pub fn parse_type_alias(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
) -> (Pipeline, Option<(Vec<u8>, TypeAliasId)>) {
    let spans = &lite_command.parts;
    let split_id = if spans.len() > 1 && working_set.get_span_contents(spans[0]) == b"export" {
        2
    } else {
        1
    };
    let (command_spans, rest_spans) = spans.split_at(split_id);
    let head = Span::concat(command_spans);
    let call_span = Span::concat(spans);

    if let Some(redirection) = lite_command.redirection.as_ref() {
        working_set.error(redirecting_builtin_error("type", redirection));
        return (garbage_pipeline(working_set, spans), None);
    }

    let Some(decl_id) = find_keyword_decl(working_set, b"type") else {
        working_set.error(ParseError::UnknownState(
            "internal error: 'type' declaration not found".into(),
            call_span,
        ));
        return (garbage_pipeline(working_set, spans), None);
    };

    // Parse the name and `=` as a call only to see whether it asks for help, as `parse_alias`
    // does. Its errors are dropped because the layout is checked below.
    let starting_error_count = working_set.parse_errors.len();
    let ParsedInternalCall {
        call,
        output,
        call_kind,
    } = parse_internal_call(
        working_set,
        head,
        &rest_spans[..rest_spans.len().min(2)],
        decl_id,
        ArgumentParsingLevel::Full,
        None,
    );
    working_set.parse_errors.truncate(starting_error_count);

    if call_kind == CallKind::Help {
        return (
            Pipeline::from_vec(vec![Expression::new(
                working_set,
                Expr::Call(call),
                call_span,
                output,
            )]),
            None,
        );
    }

    let keyword = String::from_utf8_lossy(working_set.get_span_contents(head)).into_owned();
    // `type ls` is how Bash and cmd.exe look up a command, so a bare `type` without `=` also
    // points to `which`.
    let missing_equals = |name_span: Span| {
        let name = String::from_utf8_lossy(working_set.get_span_contents(name_span));
        let lookup = if keyword == "type" {
            format!(
                " To see what the command `{name}` is, use `which {name}`. To run an external `type` program, use `^type {name}`."
            )
        } else {
            String::new()
        };
        let help =
            format!("`{keyword}` defines a type alias: `{keyword} {name} = <type>`.{lookup}");
        ParseError::TypeAliasMissingEquals {
            help,
            span: name_span,
        }
    };
    let layout_error = match rest_spans {
        [] => Some(ParseError::MissingPositional(
            "name".into(),
            Span::point(head.end),
            format!("{keyword} <name> = <type>"),
        )),
        [first, ..] if working_set.get_span_contents(*first) == b"=" => {
            Some(ParseError::AssignmentMismatch(
                format!("{keyword} missing name"),
                "missing name".into(),
                *first,
            ))
        }
        [word] => {
            let contents = String::from_utf8_lossy(working_set.get_span_contents(*word));
            Some(match contents.split_once('=') {
                Some((name, ty)) => ParseError::IncorrectValue(
                    "type alias".into(),
                    *word,
                    format!("Make sure to put spaces around '=': {keyword} {name} = {ty}"),
                ),
                None => missing_equals(*word),
            })
        }
        [name, equals, ..] if working_set.get_span_contents(*equals) != b"=" => {
            Some(missing_equals(*name))
        }
        _ => None,
    };
    if let Some(error) = layout_error {
        working_set.error(error);
        return (garbage_pipeline(working_set, spans), None);
    }
    let (name_span, equals_span, type_spans) = (rest_spans[0], rest_spans[1], &rest_spans[2..]);

    let name = working_set.get_span_contents(name_span).to_vec();
    if BUILTIN_TYPE_NAMES.contains(&name.as_slice()) {
        working_set.error(ParseError::TypeAliasShadowsBuiltin(
            String::from_utf8_lossy(&name).into_owned(),
            name_span,
        ));
        return (garbage_pipeline(working_set, spans), None);
    }
    if !is_valid_type_alias_name(&name) {
        working_set.error(ParseError::TypeAliasNameNotValid(name_span));
        return (garbage_pipeline(working_set, spans), None);
    }

    let (type_span, shape) = if type_spans.is_empty() {
        working_set.error(ParseError::MissingType(equals_span));
        (Span::point(equals_span.end), SyntaxShape::Any)
    } else {
        // A generic type such as `record<a: int, b: string>` is split into several spans at its
        // spaces. Join them and read them again as one type, as a `let` annotation does.
        let type_span = Span::concat(type_spans);
        let type_bytes = working_set.get_span_contents(type_span).to_vec();
        let (tokens, error) = lex_signature(&type_bytes, type_span.start, &[], b",", true);

        let parsed = match (error, tokens.as_slice()) {
            (Some(error), _) => {
                working_set.error(error);
                None
            }
            (None, [token]) => {
                let bytes = working_set.get_span_contents(token.span).to_vec();
                let starting_error_count = working_set.parse_errors.len();
                let parsed = parse_shape_name(working_set, &bytes, token.span);
                (working_set.parse_errors.len() == starting_error_count).then_some(parsed)
            }
            (None, [_, extra, ..]) => {
                working_set.error(ParseError::ExtraTokens(Span::new(
                    extra.span.start,
                    type_span.end,
                )));
                None
            }
            (None, []) => {
                working_set.error(ParseError::MissingType(type_span));
                None
            }
        };
        (type_span, parsed.unwrap_or(SyntaxShape::Any))
    };

    let type_alias_id = working_set.add_type_alias(TypeAlias {
        name: String::from_utf8_lossy(&name).into_owned(),
        shape,
        span: call_span,
    });

    // Nothing reads these arguments at run time; they are there for `ast`, highlighting and
    // `--help`, which show the statement as written.
    let mut call = Call::new(head);
    call.decl_id = decl_id;
    call.add_positional(Expression::new(
        working_set,
        Expr::String(String::from_utf8_lossy(&name).into_owned()),
        name_span,
        Type::String,
    ));
    let type_text = Expression::new(
        working_set,
        Expr::String(
            String::from_utf8_lossy(working_set.get_span_contents(type_span)).into_owned(),
        ),
        type_span,
        Type::String,
    );
    call.add_positional(Expression::new(
        working_set,
        Expr::Keyword(Box::new(Keyword {
            keyword: b"=".as_slice().into(),
            span: equals_span,
            expr: type_text,
        })),
        equals_span.merge(type_span),
        Type::String,
    ));

    (
        Pipeline::from_vec(vec![Expression::new(
            working_set,
            Expr::Call(Box::new(call)),
            call_span,
            Type::Nothing,
        )]),
        Some((name, type_alias_id)),
    )
}
