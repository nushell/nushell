use crate::{
    lite_parser::LiteCommand,
    parse_helpers::garbage_pipeline,
    parse_shape_specs::parse_shape_name,
};
use nu_protocol::{
    ParseError, Span, TypeDef, TypeDefKind,
    ast::{Expr, Expression, Pipeline},
    engine::StateWorkingSet,
};
use std::sync::Arc;

/// Type names that cannot be redefined by a `type` declaration, matching the
/// built-in names handled by [`parse_shape_name`].
const BUILTIN_TYPE_NAMES: &[&[u8]] = &[
    b"any",
    b"binary",
    b"block",
    b"bool",
    b"cell-path",
    b"closure",
    b"datetime",
    b"directory",
    b"duration",
    b"error",
    b"external_arg",
    b"float",
    b"filesize",
    b"glob",
    b"int",
    b"nothing",
    b"number",
    b"path",
    b"range",
    b"string",
    b"oneof",
    b"list",
    b"record",
    b"table",
    b"enum",
];

fn is_valid_type_name(name: &[u8]) -> bool {
    !name.is_empty()
        && name
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

/// The declared name and definition, for module export bookkeeping.
type ParsedType = Option<(Vec<u8>, Arc<TypeDef>)>;

/// Parse a `type` declaration:
///
/// ```nu
/// type Point = record<x: int, y: int>
/// type UserId = int
/// ```
///
/// Returns the pipeline (always a no-op `nothing` element) and, on success, the
/// declared name and definition for module export bookkeeping.
pub fn parse_type_decl(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
) -> (Pipeline, ParsedType) {
    let spans = lite_command.command_parts();

    if working_set.get_span_contents(spans[0]) != b"type" {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for 'type' command".into(),
            Span::concat(spans),
        ));
        return (garbage_pipeline(working_set, spans), None);
    }

    if lite_command.redirection.is_some() {
        working_set.error(ParseError::LabeledError(
            "type declarations cannot be redirected".into(),
            "redirect not allowed here".into(),
            Span::concat(spans),
        ));
        return (garbage_pipeline(working_set, spans), None);
    }

    // `type <name> = <definition>`
    if spans.len() < 2 {
        working_set.error(ParseError::MissingPositional(
            "type name".into(),
            Span::new(spans[0].end, spans[0].end),
            "type <name> = <type>".into(),
        ));
        return (garbage_pipeline(working_set, spans), None);
    }

    let name = working_set.get_span_contents(spans[1]).to_vec();

    if !is_valid_type_name(&name) {
        working_set.error(ParseError::IncorrectValue(
            "type name".into(),
            spans[1],
            "type names may only contain letters, digits, `-` and `_`".into(),
        ));
        return (garbage_pipeline(working_set, spans), None);
    }

    if BUILTIN_TYPE_NAMES.contains(&name.as_slice()) {
        working_set.error(ParseError::IncorrectValue(
            "type name".into(),
            spans[1],
            format!(
                "`{}` is a built-in type name and cannot be redefined",
                String::from_utf8_lossy(&name)
            ),
        ));
        return (garbage_pipeline(working_set, spans), None);
    }

    if spans.len() < 3 || working_set.get_span_contents(spans[2]) != b"=" {
        working_set.error(ParseError::AssignmentMismatch(
            "type missing sign".into(),
            "missing equal sign".into(),
            *spans
                .get(2)
                .unwrap_or(&Span::new(spans[1].end, spans[1].end)),
        ));
        return (garbage_pipeline(working_set, spans), None);
    }

    if spans.len() < 4 {
        working_set.error(ParseError::MissingPositional(
            "type definition".into(),
            Span::new(spans[2].end, spans[2].end),
            "type <name> = <type>".into(),
        ));
        return (garbage_pipeline(working_set, spans), None);
    }

    let rhs_span = Span::concat(&spans[3..]);
    let rhs = working_set.get_span_contents(rhs_span).to_vec();

    let kind = TypeDefKind::Alias(parse_shape_name(working_set, &rhs, rhs_span));

    let type_def = Arc::new(TypeDef {
        name: name.clone(),
        kind,
    });

    working_set.add_type(name.clone(), (*type_def).clone());

    let pipeline = Pipeline::from_vec(vec![Expression::new(
        working_set,
        Expr::Nothing,
        Span::concat(spans),
        nu_protocol::Type::Nothing,
    )]);

    (pipeline, Some((name, type_def)))
}
