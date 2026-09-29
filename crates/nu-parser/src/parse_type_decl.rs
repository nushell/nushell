use crate::{
    lite_parser::LiteCommand, parse_helpers::garbage_pipeline, parse_shape_specs::parse_shape_name,
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
    b"struct",
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
/// struct Point { x: int, y: int }
/// struct UserId = int
/// ```
///
/// Returns the pipeline (always a no-op `nothing` element) and, on success, the
/// declared name and definition for module export bookkeeping.
pub fn parse_type_decl(
    working_set: &mut StateWorkingSet,
    lite_command: &LiteCommand,
) -> (Pipeline, ParsedType) {
    let spans = lite_command.command_parts();

    let keyword = working_set.get_span_contents(spans[0]);
    let is_enum_decl = keyword == b"enum";
    if keyword != b"struct" && !is_enum_decl {
        working_set.error(ParseError::UnknownState(
            "internal error: Wrong call name for 'struct'/'enum' declaration".into(),
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

    let pos = 2;
    let next_bytes = if spans.len() > pos {
        working_set.get_span_contents(spans[pos])
    } else {
        b""
    };

    let (rhs, rhs_span): (Vec<u8>, Span) =
        if next_bytes.starts_with(b"{") && next_bytes.ends_with(b"}") {
            // `enum Name { ... }` / `struct Name { ... }` — the brace body is one
            // lite-parse span; wrap its interior in `enum<>`/`record<>` bytes so
            // the rest of the parser is unchanged. The synthesized span is
            // shifted by the wrapper length so interior tokens keep their real
            // file positions.
            let brace_span = Span::concat(&spans[pos..]);
            let inner = Span::new(brace_span.start + 1, brace_span.end - 1);
            let inner_bytes = working_set.get_span_contents(inner);
            let mut bytes = if is_enum_decl {
                b"enum<".to_vec()
            } else {
                b"record<".to_vec()
            };
            let offset = bytes.len();
            bytes.extend_from_slice(inner_bytes);
            bytes.push(b'>');
            (bytes, Span::new(inner.start - offset, inner.end + 1))
        } else if is_enum_decl {
            working_set.error(ParseError::LabeledError(
                "enum declarations use `enum <name> { <variants> }`".into(),
                "expected a `{` body".into(),
                *spans
                    .get(pos)
                    .unwrap_or(&Span::new(spans[pos - 1].end, spans[pos - 1].end)),
            ));
            return (garbage_pipeline(working_set, spans), None);
        } else {
            if next_bytes != b"=" {
                working_set.error(ParseError::AssignmentMismatch(
                    "type missing sign".into(),
                    "missing equal sign or `{` body".into(),
                    *spans
                        .get(pos)
                        .unwrap_or(&Span::new(spans[pos - 1].end, spans[pos - 1].end)),
                ));
                return (garbage_pipeline(working_set, spans), None);
            }
            if spans.len() <= pos + 1 {
                working_set.error(ParseError::MissingPositional(
                    "type definition".into(),
                    Span::new(spans[pos].end, spans[pos].end),
                    "struct <name> { <fields> } or struct <name> = <type>".into(),
                ));
                return (garbage_pipeline(working_set, spans), None);
            }
            let rhs_span = Span::concat(&spans[pos + 1..]);
            let rhs = working_set.get_span_contents(rhs_span).to_vec();
            if rhs.starts_with(b"enum") {
                working_set.error(ParseError::LabeledError(
                    "enum declarations use `enum <name> { <variants> }`".into(),
                    "`enum<...>` is not a type shape".into(),
                    rhs_span,
                ));
                return (garbage_pipeline(working_set, spans), None);
            }
            (rhs, rhs_span)
        };

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
