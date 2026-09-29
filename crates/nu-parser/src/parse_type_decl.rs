use crate::{
    lex::lex_signature,
    lite_parser::LiteCommand,
    parse_helpers::{garbage, garbage_pipeline},
    parse_shape_specs::{parse_shape_name, parse_type, substitute_type_vars},
};
use nu_protocol::{
    CompareTypes, DeclId, EnumDef, EnumVariant, ParseError, Span, SyntaxShape, Type, TypeDef,
    TypeDefKind, TypeSet, Value,
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

/// Split `inner` — the contents between `<` and `>` — at commas that are not
/// nested inside further angle brackets. Returns `None` on unbalanced
/// brackets or empty parts.
pub fn split_top_level_commas(inner: &[u8]) -> Option<Vec<&[u8]>> {
    let mut parts = vec![];
    let mut depth = 0i32;
    let mut start = 0;

    for (i, &b) in inner.iter().enumerate() {
        match b {
            b'<' => depth += 1,
            b'>' => depth -= 1,
            b',' if depth == 0 => {
                parts.push(&inner[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        if depth < 0 {
            return None;
        }
    }
    parts.push(&inner[start..]);

    if depth != 0 || parts.iter().any(|part| part.is_empty()) {
        return None;
    }
    Some(parts)
}

/// Split `Name<T, U>` into its base name and the raw names inside `<...>`.
/// Bare `Name` returns `(Name, [])`. Returns `None` when the text contains
/// `<`/`>` that do not form a well-formed trailing parameter list.
fn split_generic_name(bytes: &[u8]) -> Option<(&[u8], Vec<&[u8]>)> {
    match bytes.iter().position(|&b| b == b'<') {
        None => {
            if bytes.contains(&b'>') {
                None
            } else {
                Some((bytes, vec![]))
            }
        }
        Some(lt) => {
            let base = &bytes[..lt];
            let inner = &bytes[lt + 1..];
            if base.is_empty() || !inner.ends_with(b">") {
                return None;
            }
            let params = split_top_level_commas(&inner[..inner.len() - 1])?;
            if params
                .iter()
                .any(|p| p.contains(&b'<') || p.contains(&b'>') || p.is_empty())
            {
                return None;
            }
            Some((base, params))
        }
    }
}

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

    // Generic declarations (`type Pair<A, B> = ...`) may have spaces inside
    // the parameter list, which the lite parser splits across spans — rejoin
    // spans until the angle brackets balance.
    let mut name_end = 1;
    let name = {
        let mut name = working_set.get_span_contents(spans[1]).to_vec();
        let mut depth = name.iter().filter(|&&b| b == b'<').count() as isize
            - name.iter().filter(|&&b| b == b'>').count() as isize;
        while depth > 0 && name_end + 1 < spans.len() {
            name_end += 1;
            let next = working_set.get_span_contents(spans[name_end]);
            depth += next.iter().filter(|&&b| b == b'<').count() as isize
                - next.iter().filter(|&&b| b == b'>').count() as isize;
            name.extend_from_slice(next);
        }
        name
    };

    // `type Name<T, U> = ...` — generic type parameters. The list is optional;
    // names must be non-empty, unique, and must not shadow built-in type names.
    let (name, params) = match split_generic_name(&name) {
        Some((base, params)) => {
            for param in &params {
                let param_str = String::from_utf8_lossy(param);
                if !is_valid_type_name(param) {
                    working_set.error(ParseError::IncorrectValue(
                        "type parameter name".into(),
                        spans[1],
                        "type parameter names may only contain letters, digits, `-` and `_`".into(),
                    ));
                    return (garbage_pipeline(working_set, spans), None);
                }
                if BUILTIN_TYPE_NAMES.contains(param) {
                    working_set.error(ParseError::IncorrectValue(
                        "type parameter name".into(),
                        spans[1],
                        format!("`{param_str}` is a built-in type name"),
                    ));
                    return (garbage_pipeline(working_set, spans), None);
                }
            }
            if params
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != params.len()
            {
                working_set.error(ParseError::IncorrectValue(
                    "type parameter name".into(),
                    spans[1],
                    "duplicate type parameter name".into(),
                ));
                return (garbage_pipeline(working_set, spans), None);
            }
            (
                base.to_vec(),
                params
                    .iter()
                    .map(|p| String::from_utf8_lossy(p).to_string())
                    .collect::<Vec<String>>(),
            )
        }
        None => {
            working_set.error(ParseError::IncorrectValue(
                "type name".into(),
                spans[1],
                "expected `Name` or `Name<T, U>`".into(),
            ));
            return (garbage_pipeline(working_set, spans), None);
        }
    };

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

    if spans.len() < name_end + 2 || working_set.get_span_contents(spans[name_end + 1]) != b"=" {
        working_set.error(ParseError::AssignmentMismatch(
            "type missing sign".into(),
            "missing equal sign".into(),
            *spans
                .get(name_end + 1)
                .unwrap_or(&Span::new(spans[name_end].end, spans[name_end].end)),
        ));
        return (garbage_pipeline(working_set, spans), None);
    }

    if spans.len() < name_end + 3 {
        working_set.error(ParseError::MissingPositional(
            "type definition".into(),
            Span::new(spans[name_end + 1].end, spans[name_end + 1].end),
            "type <name> = <type>".into(),
        ));
        return (garbage_pipeline(working_set, spans), None);
    }

    let rhs_span = Span::concat(&spans[name_end + 2..]);
    let rhs = working_set.get_span_contents(rhs_span).to_vec();

    // Type parameters are in scope only while the right-hand side is parsed —
    // `some: T` in `enum<some: T, none>` resolves to `SyntaxShape::TypeVar`.
    let saved_params = std::mem::replace(&mut working_set.type_params, params.clone());

    let kind = if rhs == b"enum" || rhs.starts_with(b"enum<") {
        match parse_enum_def(working_set, &rhs, rhs_span) {
            Some(enum_def) => TypeDefKind::Enum(enum_def),
            None => {
                working_set.type_params = saved_params;
                return (garbage_pipeline(working_set, spans), None);
            }
        }
    } else {
        TypeDefKind::Alias(parse_shape_name(working_set, &rhs, rhs_span))
    };

    working_set.type_params = saved_params;

    let type_def = Arc::new(TypeDef {
        name: name.clone(),
        params,
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

        let has_payload =
            idx < tokens.len() && working_set.get_span_contents(tokens[idx].span) == b":";
        let payload = if has_payload {
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
    // Generic constructor heads (`Pair<int, string>.pair`) may contain spaces
    // inside the argument list, which split the head across spans — rejoin
    // spans until the angle brackets balance. Joined without separators, the
    // spans recover the intended `Name<args>.variant` token.
    let mut head = working_set.get_span_contents(head_span).to_vec();
    let mut arg_spans = arg_spans;
    let mut joined_end = head_span.end;
    let mut depth = head.iter().filter(|&&b| b == b'<').count() as isize
        - head.iter().filter(|&&b| b == b'>').count() as isize;
    while depth > 0 {
        let Some((next, rest)) = arg_spans.split_first() else {
            break;
        };
        let next_bytes = working_set.get_span_contents(*next);
        depth += next_bytes.iter().filter(|&&b| b == b'<').count() as isize
            - next_bytes.iter().filter(|&&b| b == b'>').count() as isize;
        head.extend_from_slice(next_bytes);
        joined_end = next.end;
        arg_spans = rest;
    }

    let dot = head.iter().rposition(|&b| b == b'.')?;
    let type_name = &head[..dot];
    let variant_name = &head[dot + 1..];
    if type_name.is_empty() || variant_name.is_empty() {
        return None;
    }

    // `Name<args>` — an explicitly instantiated generic enum type. The base
    // name is resolved and the arguments bind its type parameters. Arguments
    // are re-sliced from the source range covering the joined head (which
    // keeps whitespace that `head` loses when rejoining) so nested shapes
    // like `record<a: int>` get correctly positioned spans for sub-lexing.
    let head_file = working_set
        .get_span_contents(Span::new(head_span.start, joined_end))
        .to_vec();
    let (base_name, type_args) = match type_name.iter().position(|&b| b == b'<') {
        Some(_) => {
            let lt = head_file.iter().position(|&b| b == b'<')?;
            let mut depth = 0isize;
            let mut close = None;
            for (i, &b) in head_file[lt..].iter().enumerate() {
                match b {
                    b'<' => depth += 1,
                    b'>' => {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(lt + i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let close = close?;
            let arg_src = &head_file[lt + 1..close];
            let arg_src_start = head_span.start + lt + 1;
            let args = split_top_level_commas(arg_src)?;
            let args: Vec<(&[u8], Span)> = args
                .iter()
                .map(|arg| {
                    let arg = arg.trim_ascii();
                    let offset = arg.as_ptr() as usize - arg_src.as_ptr() as usize;
                    (
                        arg,
                        Span::new(arg_src_start + offset, arg_src_start + offset + arg.len()),
                    )
                })
                .collect();
            (&type_name[..head.iter().position(|&b| b == b'<')?], args)
        }
        None => (type_name, vec![]),
    };

    let type_def = working_set.find_type_name(base_name)?;
    let TypeDefKind::Enum(enum_def) = &type_def.kind else {
        return None;
    };

    let variant_name_str = String::from_utf8_lossy(variant_name).to_string();
    let type_name_str = String::from_utf8_lossy(&type_def.name).to_string();
    // The name as written is what the runtime command resolves; the
    // canonical `type_def.name` is what values and signature checks report.
    // Generic arguments are erased at runtime.
    let type_path_str = String::from_utf8_lossy(base_name).to_string();

    if !type_args.is_empty() && type_args.len() != type_def.params.len() {
        working_set.error(ParseError::IncorrectValue(
            "type arguments".into(),
            head_span,
            format!(
                "`{type_name_str}` takes {} type argument(s), but {} were supplied",
                type_def.params.len(),
                type_args.len()
            ),
        ));
        return Some(garbage(working_set, call_span));
    }

    let arg_shapes: Vec<SyntaxShape> = type_args
        .iter()
        .map(|(arg, arg_span)| parse_shape_name(working_set, arg, *arg_span))
        .collect();

    let bindings: std::collections::HashMap<&str, &SyntaxShape> = type_def
        .params
        .iter()
        .map(String::as_str)
        .zip(arg_shapes.iter())
        .collect();

    // The static name of the constructed value: `Option<int>` when
    // instantiated, the bare name otherwise (possibly refined by payload
    // inference below).
    let instantiated_name = |arg_types: &[Type]| type_def.instantiated_name(arg_types);
    let explicit_name = |ty_def: &TypeDef| {
        ty_def.instantiated_name(
            &arg_shapes
                .iter()
                .map(SyntaxShape::to_type)
                .collect::<Vec<_>>(),
        )
    };

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
            Type::Custom(explicit_name(&type_def).into()),
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

    // The payload shape with the declared type parameters bound: explicit
    // arguments substitute (`Option<int>.some` checks `T` against `int`);
    // a bare generic constructor treats parameters as `any` and infers them
    // from the payload's parsed type (`Option.some 5` → `Option<int>`).
    let bound_payload = |payload_shape: &SyntaxShape| -> SyntaxShape {
        if bindings.is_empty() {
            let any = SyntaxShape::Any;
            let any_bindings: std::collections::HashMap<&str, &SyntaxShape> = type_def
                .params
                .iter()
                .map(String::as_str)
                .map(|param| (param, &any))
                .collect();
            substitute_type_vars(payload_shape, &any_bindings)
        } else {
            substitute_type_vars(payload_shape, &bindings)
        }
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
                let bound = bound_payload(payload_shape);
                let expr = crate::parser::parse_value(working_set, *arg_span, &bound, input_type);

                let expected = bound.to_type();
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

    // Infer the instantiation from the payload when no `<...>` was written.
    let output_name = if !type_args.is_empty() {
        explicit_name(&type_def)
    } else if type_def.params.is_empty() {
        instantiated_name(&[])
    } else {
        match (&variant.payload, &payload_expr) {
            (Some(payload_shape), Some(expr)) => {
                let mut inferred = std::collections::HashMap::new();
                infer_type_args(
                    working_set,
                    payload_shape,
                    &expr.ty,
                    head_span,
                    &mut inferred,
                );
                let arg_types: Vec<Type> = type_def
                    .params
                    .iter()
                    .map(|param| inferred.get(param).cloned().unwrap_or(Type::Any))
                    .collect();
                instantiated_name(&arg_types)
            }
            _ => instantiated_name(&[]),
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
        Type::Custom(output_name.into()),
        head_span,
        call_span,
        args,
    )
}

/// Collect bindings for generic type parameters by matching a declared
/// payload `shape` (which may contain [`SyntaxShape::TypeVar`]) against the
/// parsed payload `ty`. Used to infer the instantiation of a bare generic
/// enum constructor (`Option.some 5` binds `T = int`).
fn infer_type_args(
    working_set: &mut StateWorkingSet,
    shape: &SyntaxShape,
    ty: &Type,
    span: Span,
    bindings: &mut std::collections::HashMap<String, Type>,
) {
    match (shape, ty) {
        (SyntaxShape::TypeVar(name), ty) => {
            bindings
                .entry(name.to_string())
                .and_modify(|bound| *bound = bound.clone().union(ty.clone()))
                .or_insert_with(|| ty.clone());
        }
        (SyntaxShape::Named(_, inner), _) => {
            infer_type_args(working_set, inner, ty, span, bindings)
        }
        (SyntaxShape::Custom(_, arg_shapes), Type::Custom(name)) => {
            if let Some(lt) = name.find('<')
                && let Some(arg_bytes) = name[lt + 1..]
                    .strip_suffix('>')
                    .map(str::as_bytes)
                    .and_then(split_top_level_commas)
            {
                for (arg_shape, arg_bytes) in arg_shapes.iter().zip(arg_bytes) {
                    let arg_ty = parse_type(working_set, arg_bytes, span);
                    infer_type_args(working_set, arg_shape, &arg_ty, span, bindings);
                }
            }
        }
        (SyntaxShape::List(inner), Type::List(elem)) => {
            infer_type_args(working_set, inner, elem, span, bindings)
        }
        (SyntaxShape::OneOf(shapes), _) => {
            for shape in shapes {
                infer_type_args(working_set, shape, ty, span, bindings);
            }
        }
        (SyntaxShape::Record(rows), Type::Record(cols))
        | (SyntaxShape::Table(rows), Type::Table(cols)) => {
            for (name, row_shape) in rows.iter() {
                if let Some(col_ty) = cols.get(name) {
                    infer_type_args(working_set, row_shape, col_ty, span, bindings);
                }
            }
        }
        (SyntaxShape::Keyword(_, inner), _) => {
            infer_type_args(working_set, inner, ty, span, bindings)
        }
        _ => {}
    }
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
/// the pattern and the variant's declared payload shape, if any. `None`
/// when `span` is not `<known enum type>.<variant>`; an unknown variant on
/// a known enum type is a parse error.
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

    let type_def = working_set.find_type_name(type_base_name(type_name))?;
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

/// The base name of a possibly-instantiated type spelling — `Option<int>` →
/// `Option`. Type arguments are erased, so exhaustiveness and variant lookup
/// always happen against the base declaration.
fn type_base_name(name: &[u8]) -> &[u8] {
    match name.iter().position(|&b| b == b'<') {
        Some(lt) => &name[..lt],
        None => name,
    }
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

    let type_def = working_set.find_type_name(type_base_name(type_name.as_bytes()))?;
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
