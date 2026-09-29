use crate::{
    lex::lex_signature,
    lite_parser::LiteCommand,
    parse_helpers::{garbage, garbage_pipeline},
    parse_shape_specs::parse_shape_name,
};
use nu_protocol::{
    CompareTypes, EnumDef, EnumVariant, ParseError, Span, Type, TypeDef, TypeDefKind,
    ast::{Argument, Call, Expr, Expression, Pipeline},
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
/// enum Shape { circle: record<radius: float>, point }
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

    let kind = if rhs == b"enum" || rhs.starts_with(b"enum<") {
        match parse_enum_def(working_set, &rhs, rhs_span) {
            Some(enum_def) => TypeDefKind::Enum(enum_def),
            None => return (garbage_pipeline(working_set, spans), None),
        }
    } else {
        TypeDefKind::Alias(parse_shape_name(working_set, &rhs, rhs_span))
    };

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

fn is_valid_variant_name(name: &[u8]) -> bool {
    !name.is_empty()
        && name
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

/// Parse the `enum<variant: <payload-shape>, ...>` definition body.
fn parse_enum_def(working_set: &mut StateWorkingSet, bytes: &[u8], span: Span) -> Option<EnumDef> {
    if !bytes.starts_with(b"enum") {
        working_set.error(ParseError::Expected("`enum`", span));
        return None;
    }

    let inner = &bytes[b"enum".len()..];
    if !(inner.starts_with(b"<") && inner.ends_with(b">")) {
        working_set.error(ParseError::Expected(
            "enum variant list like `{ a: int, b }`",
            Span::new(span.start + b"enum".len(), span.end),
        ));
        return None;
    }

    let inner_span = Span::new(span.start + b"enum".len() + 1, span.end - 1);
    let inner_src = working_set.get_span_contents(inner_span);

    let (tokens, err) = lex_signature(inner_src, inner_span.start, b"\n\r", b":,", true);
    if let Some(err) = err {
        working_set.error(err);
        return None;
    }

    let mut variants = vec![];
    let mut idx = 0;

    while idx < tokens.len() {
        let token = &tokens[idx];
        let contents = working_set.get_span_contents(token.span);

        if contents == b"," {
            idx += 1;
            continue;
        }

        if !is_valid_variant_name(contents) {
            working_set.error(ParseError::IncorrectValue(
                "variant name".into(),
                token.span,
                "variant names may only contain letters, digits, `-` and `_`".into(),
            ));
            return None;
        }

        let name = String::from_utf8_lossy(contents).to_string();
        idx += 1;

        let payload =
            if idx < tokens.len() && working_set.get_span_contents(tokens[idx].span) == b":" {
                idx += 1;
                if idx >= tokens.len() {
                    working_set.error(ParseError::Expected("type after colon", token.span));
                    return None;
                }
                let shape_span = tokens[idx].span;
                let shape_bytes = working_set.get_span_contents(shape_span).to_vec();
                let shape = parse_shape_name(working_set, &shape_bytes, shape_span);
                idx += 1;
                Some(shape)
            } else {
                None
            };

        if variants.iter().any(|v: &EnumVariant| v.name == name) {
            working_set.error(ParseError::IncorrectValue(
                "variant name".into(),
                token.span,
                format!("duplicate variant `{name}`"),
            ));
            return None;
        }

        variants.push(EnumVariant { name, payload });
    }

    Some(EnumDef { variants })
}
///
/// `Type.variant [payload]` parses to `enum-construct <type> <variant>
/// [payload]`; the command validates the payload again at runtime.
pub const ENUM_CONSTRUCT_DECL: &[u8] = b"enum-construct";

/// Try to parse `Type.variant [payload]` as an enum constructor call.
///
/// Returns `None` when `head` is not `<known enum type>.<name>` (e.g. `Shape`
/// doesn't resolve to an `enum` declaration) so the caller can fall back to
/// normal command resolution. Parse errors are emitted and a garbage
/// expression is returned in `Some`.
pub fn parse_enum_constructor(
    working_set: &mut StateWorkingSet,
    head_span: Span,
    arg_spans: &[Span],
    call_span: Span,
    input_type: Option<&Type>,
) -> Option<Expression> {
    let head = working_set.get_span_contents(head_span);

    let dot = head.iter().rposition(|&b| b == b'.')?;
    let type_name = &head[..dot];
    let variant_name = &head[dot + 1..];
    if type_name.is_empty() || variant_name.is_empty() {
        return None;
    }

    let type_def = working_set.find_type_name(type_name)?;
    let TypeDefKind::Enum(enum_def) = &type_def.kind else {
        return None;
    };

    let variant_name_str = String::from_utf8_lossy(variant_name).to_string();
    let type_name_str = String::from_utf8_lossy(&type_def.name).to_string();

    let Some(variant) = enum_def.get_variant(&variant_name_str) else {
        working_set.error(ParseError::LabeledErrorWithHelp {
            error: format!("unknown variant `{variant_name_str}`"),
            label: format!("not a variant of `{type_name_str}`"),
            help: format!(
                "variants of `{type_name_str}`: {}",
                enum_def.variant_names().join(", ")
            ),
            span: head_span,
        });
        return Some(garbage(working_set, call_span));
    };

    let decl_id = working_set.find_decl(ENUM_CONSTRUCT_DECL)?;

    // Validate the payload argument statically where possible.
    let payload_expr = match &variant.payload {
        Some(payload_shape) => match arg_spans {
            [] => {
                working_set.error(ParseError::MissingPositional(
                    format!("payload for `{variant_name_str}`"),
                    Span::new(call_span.end, call_span.end),
                    format!(
                        "{type_name_str}.{variant_name_str} <{}>",
                        payload_shape.to_type()
                    ),
                ));
                return Some(garbage(working_set, call_span));
            }
            [arg_span, extra @ ..] => {
                let expr =
                    crate::parser::parse_value(working_set, *arg_span, payload_shape, input_type);

                let expected = payload_shape.to_type();
                if !matches!(expr.ty, Type::Any | Type::Error) && !expr.ty.is_subtype_of(&expected)
                {
                    working_set.error(ParseError::TypeMismatch(
                        expected.clone(),
                        expr.ty.clone(),
                        *arg_span,
                    ));
                }

                if let [first, ..] = extra {
                    working_set.error(ParseError::ExtraPositional(
                        format!("{type_name_str}.{variant_name_str} <payload>"),
                        *first,
                    ));
                }

                Some(expr)
            }
        },
        None => {
            if let [first, ..] = arg_spans {
                working_set.error(ParseError::ExtraPositional(
                    format!("{type_name_str}.{variant_name_str} takes no payload"),
                    *first,
                ));
            }
            None
        }
    };

    let mut call = Call::new(call_span);
    call.decl_id = decl_id;
    call.head = head_span;
    call.arguments.push(Argument::Positional(Expression::new(
        working_set,
        Expr::String(type_name_str.clone()),
        head_span,
        Type::String,
    )));
    call.arguments.push(Argument::Positional(Expression::new(
        working_set,
        Expr::String(variant_name_str),
        head_span,
        Type::String,
    )));
    if let Some(payload) = payload_expr {
        call.arguments.push(Argument::Positional(payload));
    }

    Some(Expression::new(
        working_set,
        Expr::Call(Box::new(call)),
        call_span,
        Type::Custom(type_name_str.into()),
    ))
}
