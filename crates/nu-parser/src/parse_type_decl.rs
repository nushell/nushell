use crate::{
    lex::lex_signature,
    lite_parser::LiteCommand,
    parse_helpers::{garbage, garbage_pipeline},
    parse_shape_specs::parse_shape_name,
};
use nu_protocol::{
    CompareTypes, DeclId, EnumDef, EnumVariant, ParseError, Span, SyntaxShape, Type, TypeDef,
    TypeDefKind, Value,
    ast::{Argument, Call, Expr, Expression, MatchPattern, Pattern, Pipeline},
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

fn is_valid_variant_name(name: &[u8]) -> bool {
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
/// type Shape = enum<circle: record<radius: float>, point>
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

/// Parse the `enum<variant: <payload-shape>, ...>` definition body.
fn parse_enum_def(working_set: &mut StateWorkingSet, bytes: &[u8], span: Span) -> Option<EnumDef> {
    if !bytes.starts_with(b"enum") {
        working_set.error(ParseError::Expected("`enum`", span));
        return None;
    }

    let inner = &bytes[b"enum".len()..];
    if !(inner.starts_with(b"<") && inner.ends_with(b">")) {
        working_set.error(ParseError::Expected(
            "enum variant list like `enum<a, b: int>`",
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

/// The name of the internal command enum constructors are rewritten to.
///
/// `Type.variant [payload]` parses to `enum-construct <type> <variant>
/// [payload]`; the command validates the payload again at runtime.
pub const ENUM_CONSTRUCT_DECL: &[u8] = b"enum-construct";

/// The name of the internal command `Type.from-record` is rewritten to.
///
/// `Type.from-record <record>` parses to `enum-from-record <type> <record>`;
/// the command rebuilds an enum value from its base-record representation.
pub const ENUM_FROM_RECORD_DECL: &[u8] = b"enum-from-record";

/// Both enum commands are hidden from `help` and completion, so
/// [`StateWorkingSet::find_decl`] will not resolve them. Scan the decl table
/// by name instead — visibility only affects name lookups, not `DeclId`s.
fn find_internal_decl(working_set: &StateWorkingSet, name: &[u8]) -> Option<DeclId> {
    (0..working_set.num_decls())
        .map(DeclId::new)
        .find(|&id| working_set.get_decl(id).name().as_bytes() == name)
}

fn enum_internal_call(
    working_set: &mut StateWorkingSet,
    decl_name: &[u8],
    type_arg: &str,
    out_ty: Type,
    head_span: Span,
    call_span: Span,
    args: Vec<Argument>,
) -> Option<Expression> {
    let mut call = Call::new(call_span);
    call.decl_id = find_internal_decl(working_set, decl_name)?;
    call.head = head_span;
    call.arguments.push(Argument::Positional(Expression::new(
        working_set,
        Expr::String(type_arg.to_string()),
        head_span,
        Type::String,
    )));
    call.arguments.extend(args);
    Some(Expression::new(
        working_set,
        Expr::Call(Box::new(call)),
        call_span,
        out_ty,
    ))
}

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
    // The name as written is what the runtime command resolves; the
    // canonical `type_def.name` is what values and signature checks report.
    let type_path_str = String::from_utf8_lossy(type_name).to_string();

    // `Type.from-record` rebuilds an enum value from its base-record form —
    // the inverse of what `to json`/`to nuon` produce.
    if variant_name == b"from-record" {
        let arg_expr = match arg_spans {
            [] => {
                working_set.error(ParseError::MissingPositional(
                    format!("record for `{type_path_str}.from-record`"),
                    Span::new(call_span.end, call_span.end),
                    format!("{type_path_str}.from-record <record>"),
                ));
                return Some(garbage(working_set, call_span));
            }
            [arg_span, extra @ ..] => {
                let expr = crate::parser::parse_value(
                    working_set,
                    *arg_span,
                    &SyntaxShape::Any,
                    input_type,
                );
                if let [first, ..] = extra {
                    working_set.error(ParseError::ExtraPositional(
                        format!("{type_path_str}.from-record <record>"),
                        *first,
                    ));
                }
                expr
            }
        };

        return enum_internal_call(
            working_set,
            ENUM_FROM_RECORD_DECL,
            &type_path_str,
            Type::Custom(type_name_str.into()),
            head_span,
            call_span,
            vec![Argument::Positional(arg_expr)],
        );
    }

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

    let mut args = vec![Argument::Positional(Expression::new(
        working_set,
        Expr::String(variant_name_str),
        head_span,
        Type::String,
    ))];
    if let Some(payload) = payload_expr {
        args.push(Argument::Positional(payload));
    }

    enum_internal_call(
        working_set,
        ENUM_CONSTRUCT_DECL,
        &type_path_str,
        Type::Custom(type_name_str.into()),
        head_span,
        call_span,
        args,
    )
}

/// Which enum variants a [`Pattern`](nu_protocol::ast::Pattern) covers.
enum Coverage {
    /// The pattern matches regardless of the variant (`_`, `$x`,
    /// `{kind: $k}`, a record pattern without a `kind` constraint, ...).
    All,
    /// The pattern selects a fixed set of variant names.
    Variants(std::collections::HashSet<String>),
}

fn pattern_coverage(pattern: &nu_protocol::ast::Pattern) -> Coverage {
    use nu_protocol::ast::Pattern;
    match pattern {
        Pattern::Variable(_) | Pattern::IgnoreValue => Coverage::All,
        Pattern::Or(patterns) => {
            let mut variants = std::collections::HashSet::new();
            for pattern in patterns {
                match pattern_coverage(&pattern.pattern) {
                    Coverage::All => return Coverage::All,
                    Coverage::Variants(vs) => variants.extend(vs),
                }
            }
            Coverage::Variants(variants)
        }
        Pattern::Record(fields) => {
            for (field_name, field_pattern) in fields {
                if field_name == "kind" {
                    return match &field_pattern.pattern {
                        // `{kind: "circle"}` selects a single variant.
                        Pattern::Value(nu_protocol::Value::String { val, .. }) => {
                            Coverage::Variants(std::collections::HashSet::from([val.clone()]))
                        }
                        // `{kind: 5}` can never match an enum value.
                        Pattern::Value(_) => Coverage::Variants(std::collections::HashSet::new()),
                        // `{kind: $k}` binds any variant; `{kind: "a" | "b"}`
                        // unions them.
                        other => pattern_coverage(other),
                    };
                }
            }
            // A record pattern with no `kind` field might match several
            // variants; conservatively treat it as covering all of them.
            Coverage::All
        }
        _ => Coverage::Variants(std::collections::HashSet::new()),
    }
}

/// Try to parse `span` as a qualified enum-variant pattern `Type.variant`.
///
/// Lowers to a record pattern on the variant tag — `S.circle` matches the
/// same way `{kind: "circle"}` does, through the enum's base record. Returns
/// `None` when `span` is not `<known enum type>.<variant>`; an unknown
/// variant on a known enum type is a parse error.
pub fn parse_enum_variant_pattern(
    working_set: &mut StateWorkingSet,
    span: Span,
) -> Option<(MatchPattern, Option<SyntaxShape>)> {
    let bytes = working_set.get_span_contents(span);
    let dot = bytes.iter().rposition(|&b| b == b'.')?;
    let type_name = &bytes[..dot];
    let variant_name = &bytes[dot + 1..];
    if type_name.is_empty() || !is_valid_variant_name(variant_name) {
        return None;
    }

    let type_def = working_set.find_type_name(type_name)?;
    let TypeDefKind::Enum(enum_def) = &type_def.kind else {
        return None;
    };

    let variant_name_str = String::from_utf8_lossy(variant_name).to_string();
    let Some(variant) = enum_def.get_variant(&variant_name_str) else {
        let type_name_str = String::from_utf8_lossy(&type_def.name);
        working_set.error(ParseError::LabeledErrorWithHelp {
            error: format!("unknown variant `{variant_name_str}`"),
            label: format!("not a variant of `{type_name_str}`"),
            help: format!(
                "variants of `{type_name_str}`: {}",
                enum_def.variant_names().join(", ")
            ),
            span,
        });
        return Some((
            MatchPattern {
                pattern: Pattern::Garbage,
                guard: None,
                span,
            },
            None,
        ));
    };

    Some((
        MatchPattern {
            pattern: Pattern::Record(vec![(
                "kind".into(),
                MatchPattern {
                    pattern: Pattern::Value(Value::string(variant_name_str, span)),
                    guard: None,
                    span,
                },
            )]),
            guard: None,
            span,
        },
        variant.payload.clone(),
    ))
}

/// Which enum variants of a declared-enum scrutinee are not covered by the
/// match arms. `None` when the scrutinee is not a declared enum type or the
/// type is out of scope.
///
/// Guards make an arm conditional, so guarded arms do not contribute
/// coverage; wildcards and variable bindings cover all variants.
pub fn enum_match_uncovered<'a>(
    working_set: &mut StateWorkingSet,
    scrutinee_ty: &Type,
    matches: impl Iterator<Item = &'a MatchPattern>,
) -> Option<Vec<String>> {
    let Type::Custom(type_name) = scrutinee_ty else {
        return None;
    };

    let type_def = working_set.find_type_name(type_name.as_bytes())?;
    let TypeDefKind::Enum(enum_def) = &type_def.kind else {
        return None;
    };

    let mut uncovered: std::collections::HashSet<&str> =
        enum_def.variants.iter().map(|v| v.name.as_str()).collect();

    for pattern in matches {
        if pattern.guard.is_some() {
            continue;
        }
        match pattern_coverage(&pattern.pattern) {
            Coverage::All => {
                uncovered.clear();
                break;
            }
            Coverage::Variants(variants) => {
                for variant in &variants {
                    uncovered.remove(variant.as_str());
                }
            }
        }
    }

    let mut uncovered: Vec<_> = uncovered.into_iter().collect();
    uncovered.sort_unstable();
    Some(uncovered.iter().map(|v| v.to_string()).collect())
}

/// Emit a parse error when a `match` on a declared enum type is not
/// exhaustive.
///
/// `match` blocks only get this check when the scrutinee is known to be a
/// declared `enum` type — anything else is skipped. Guards make an arm
/// conditional, so guarded arms do not contribute coverage.
pub fn check_enum_match_exhaustiveness<'a>(
    working_set: &mut StateWorkingSet,
    scrutinee_ty: &Type,
    matches: impl Iterator<Item = &'a MatchPattern>,
    span: Span,
) {
    let Type::Custom(type_name) = scrutinee_ty else {
        return;
    };
    let Some(uncovered) = enum_match_uncovered(working_set, scrutinee_ty, matches) else {
        return;
    };

    if !uncovered.is_empty() {
        working_set.error(ParseError::LabeledErrorWithHelp {
            error: format!("match is not exhaustive for `{type_name}`"),
            label: format!("missing variants: {}", uncovered.join(", ")),
            help: format!("cover all variants of `{type_name}` or add a `_` arm"),
            span,
        });
    }
}
