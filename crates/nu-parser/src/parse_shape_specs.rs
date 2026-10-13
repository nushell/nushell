#![allow(clippy::byte_char_slices)]

use std::borrow::Cow;

use crate::{TokenContents, lex::lex_signature, parser::parse_value};
use nu_protocol::{
    CollectionColumns, Completion, IntoSpanned, ParseError, ShellError, Span, Spanned, SyntaxShape,
    Type, Value, engine::StateWorkingSet, eval_const::eval_constant,
};
use nu_utils::NuCow;

/// The words [`parse_shape_name`] reads as built-in types, which a type alias can't be named.
pub const BUILTIN_TYPE_NAMES: &[&[u8]] = &[
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
    b"filesize",
    b"float",
    b"glob",
    b"int",
    b"list",
    b"nothing",
    b"number",
    b"oneof",
    b"path",
    b"range",
    b"record",
    b"string",
    b"table",
];

/// [`parse_shape_name`] then convert to Type
pub fn parse_type(working_set: &mut StateWorkingSet, bytes: &[u8], span: Span) -> Type {
    parse_shape_name(working_set, bytes, span).to_type()
}

/// Parse the literals of [`Type`]-like [`SyntaxShape`]s including inner types.
///
/// A word that isn't a built-in type is looked up as a type alias. A new built-in type word
/// also goes in [`BUILTIN_TYPE_NAMES`], so that no alias can take the name.
///
/// NOTE: Does not provide a mapping to every [`SyntaxShape`]
pub fn parse_shape_name(
    working_set: &mut StateWorkingSet,
    bytes: &[u8],
    span: Span,
) -> SyntaxShape {
    match bytes {
        b"any" => SyntaxShape::Any,
        b"binary" => SyntaxShape::Binary,
        b"block" => {
            working_set.error(ParseError::LabeledErrorWithHelp {
                error: "Blocks are not support as first-class values".into(),
                label: "blocks are not supported as values".into(),
                help: "Use 'closure' instead of 'block'".into(),
                span,
            });
            SyntaxShape::Any
        }
        b"bool" => SyntaxShape::Boolean,
        b"cell-path" => SyntaxShape::CellPath,
        b"closure" => SyntaxShape::Closure(None), //FIXME: Blocks should have known output types
        b"datetime" => SyntaxShape::DateTime,
        b"directory" => SyntaxShape::Directory,
        b"duration" => SyntaxShape::Duration,
        b"error" => SyntaxShape::Error,
        b"external_arg" => SyntaxShape::ExternalArgument,
        b"float" => SyntaxShape::Float,
        b"filesize" => SyntaxShape::Filesize,
        b"glob" => SyntaxShape::GlobPattern,
        b"int" => SyntaxShape::Int,
        b"nothing" => SyntaxShape::Nothing,
        b"number" => SyntaxShape::Number,
        b"path" => SyntaxShape::Filepath,
        b"range" => SyntaxShape::Range,
        b"string" => SyntaxShape::String,
        _ if matches!(type_name(bytes), b"oneof" | b"list" | b"record" | b"table") => {
            parse_generic_shape(working_set, bytes, span)
        }
        _ if bytes.contains(&b'@') => {
            working_set.error(ParseError::LabeledError(
                "Unexpected custom completer in type spec".into(),
                "Type specifications do not support custom completers".into(),
                span,
            ));
            working_set.error(ParseError::UnknownType(span));
            SyntaxShape::Any
        }
        _ => expand_type_alias(working_set, bytes, span),
    }
}

/// The part of a type before its type parameters: `list` for `list<int>`.
fn type_name(bytes: &[u8]) -> &[u8] {
    bytes
        .iter()
        .position(|b| *b == b'<')
        .map_or(bytes, |idx| &bytes[..idx])
}

/// Return what the type alias `bytes` expands to, or report why `bytes` names no type.
fn expand_type_alias(working_set: &mut StateWorkingSet, bytes: &[u8], span: Span) -> SyntaxShape {
    let name = type_name(bytes);

    if let Some(type_alias_id) = working_set.find_type_alias(name) {
        if name.len() < bytes.len() {
            working_set.error(ParseError::TypeAliasWithParams(
                String::from_utf8_lossy(name).into_owned(),
                Span::new(span.start + name.len(), span.end),
            ));
        }
        return working_set.get_type_alias(type_alias_id).shape.clone();
    }

    // A `type` statement for this name further on, recorded with the predeclarations, makes this
    // a use before the definition, or a use inside it.
    let definition = working_set
        .find_type_predecls(name)
        .find(|definition| definition.end > span.start);
    let error = match definition {
        Some(definition) if definition.contains_span(span) => {
            ParseError::TypeAliasSelfReference(String::from_utf8_lossy(name).into_owned(), span)
        }
        Some(definition) => ParseError::TypeAliasUsedBeforeDefinition(
            String::from_utf8_lossy(name).into_owned(),
            span,
            definition,
        ),
        None => ParseError::UnknownType(span),
    };
    working_set.error(error);

    SyntaxShape::Any
}

/// Handles the specification of custom completions with `type@completer`.
pub fn parse_completer(
    working_set: &mut StateWorkingSet,
    _bytes: &[u8],
    span: Span,
) -> Option<Completion> {
    let error_count = working_set.parse_errors.len();
    let expr = parse_value(
        working_set,
        span,
        &SyntaxShape::OneOf(vec![
            SyntaxShape::List(Box::new(SyntaxShape::String)),
            SyntaxShape::String,
        ]),
        None,
    );

    if working_set.parse_errors.len() > error_count {
        return None;
    }

    let val = match eval_constant(working_set, &expr) {
        Ok(val) => val,
        Err(e) => {
            working_set.error(e.wrap(working_set, span));
            return None;
        }
    };

    let completion = match val {
        // Static list completions
        Value::List { vals, .. } => vals
            .into_iter()
            .map(|val| {
                let span = val.span();
                match val {
                    // TODO: currently `Completion::List` only supports simple string suggestions,
                    // but it will likely support description and style properties as well.
                    //
                    // For that reason records with a "value" field are accepted. So one can use
                    // choose to use "full"/record suggestions even if they get no benefit from
                    // that *not*, and when support for the other properties land their completions
                    // will improve without any extra intervention.
                    Value::Record { val, .. } => val
                        .get("value")
                        .ok_or(ShellError::CantFindColumn {
                            col_name: "value".into(),
                            span: None,
                            src_span: span,
                        })
                        .and_then(Value::coerce_str)
                        .map(Cow::into_owned),
                    val => val.coerce_into_string(),
                }
            })
            .collect::<Result<Vec<_>, ShellError>>()
            .map_err(|err| err.wrap(working_set, span))
            .map(|vals| Completion::List(NuCow::Owned(vals))),

        // Command completions
        Value::String { val, .. } => working_set
            .find_decl(val.as_bytes())
            .map(Completion::Command)
            .ok_or(ParseError::UnknownCommand(span)),

        val => Err(ParseError::OperatorUnsupportedType {
            op: "parameter completer",
            unsupported: val.get_type(),
            op_span: Span::new(span.start - 1, span.start),
            unsupported_span: span,
            help: Some(
                "\
                    the completer can only be a\n\
                    - string (name of a command)\n\
                    - list of items with simple string representations\
                ",
            ),
        }),
    };

    match completion {
        Ok(completion) => Some(completion),
        Err(err) => {
            working_set.error(err);
            None
        }
    }
}

fn parse_generic_shape(
    working_set: &mut StateWorkingSet<'_>,
    bytes: &[u8],
    span: Span,
) -> SyntaxShape {
    let (type_name, type_params) = split_generic_params(working_set, bytes, span);
    match type_name {
        b"oneof" => SyntaxShape::OneOf(match type_params {
            Some(params) => parse_type_params(working_set, params),
            None => vec![],
        }),
        b"list" => SyntaxShape::List(Box::new(match type_params {
            Some(params) => {
                let mut parsed_params = parse_type_params(working_set, params);
                if parsed_params.len() > 1 {
                    working_set.error(ParseError::LabeledError(
                        "expected a single type parameter".into(),
                        "only one parameter allowed".into(),
                        params.span,
                    ));
                    SyntaxShape::Any
                } else {
                    parsed_params.pop().unwrap_or(SyntaxShape::Any)
                }
            }
            None => SyntaxShape::Any,
        })),
        b"record" => SyntaxShape::Record(
            type_params
                .map(|params| parse_named_type_params(working_set, params))
                .unwrap_or_default(),
        ),
        b"table" => SyntaxShape::Table(
            type_params
                .map(|params| parse_named_type_params(working_set, params))
                .unwrap_or_default(),
        ),
        _ => {
            working_set.error(ParseError::UnknownType(span));
            SyntaxShape::Any
        }
    }
}

fn split_generic_params<'a>(
    working_set: &mut StateWorkingSet,
    bytes: &'a [u8],
    span: Span,
) -> (&'a [u8], Option<Spanned<&'a [u8]>>) {
    let Some(open_delim_pos) = bytes.iter().position(|&c| c == b'<') else {
        return (bytes, None);
    };

    let type_name = &bytes[..(open_delim_pos)];
    let params = &bytes[(open_delim_pos + 1)..];

    let start = span.start + type_name.len() + 1;

    if params.ends_with(b">") {
        let end = span.end - 1;
        (
            type_name,
            Some((&params[..(params.len() - 1)]).into_spanned(Span::new(start, end))),
        )
    } else if let Some(close_delim_pos) = params.iter().position(|it| *it == b'>') {
        let span = Span::new(span.start + close_delim_pos, span.end);

        working_set.error(ParseError::LabeledError(
            "Extra characters in the parameter name".into(),
            "extra characters".into(),
            span,
        ));

        (bytes, None)
    } else {
        let open = ParseError::opener_span(span, 1);
        working_set.error(ParseError::unclosed(">", open, span));
        (bytes, None)
    }
}

fn parse_named_type_params(
    working_set: &mut StateWorkingSet,
    Spanned { item: source, span }: Spanned<&[u8]>,
) -> CollectionColumns<SyntaxShape> {
    let (tokens, err) = lex_signature(source, span.start, &[b'\n', b'\r'], &[b':', b','], true);

    if let Some(err) = err {
        working_set.error(err);
        return CollectionColumns::default();
    }

    let mut sig = Vec::new();
    let mut idx = 0;

    let key_error = |span| {
        ParseError::LabeledError(
            // format!("`{name}` type annotations key not string"),
            "annotation key not string".into(),
            "must be a string".into(),
            span,
        )
    };

    while idx < tokens.len() {
        let TokenContents::Item = tokens[idx].contents else {
            working_set.error(key_error(tokens[idx].span));
            return CollectionColumns::default();
        };

        if working_set
            .get_span_contents(tokens[idx].span)
            .starts_with(b",")
        {
            idx += 1;
            continue;
        }

        let Some(key) =
            parse_value(working_set, tokens[idx].span, &SyntaxShape::String, None).as_string()
        else {
            working_set.error(key_error(tokens[idx].span));
            return CollectionColumns::default();
        };

        // we want to allow such an annotation
        // `record<name>` where the user leaves out the type
        if idx + 1 == tokens.len() {
            sig.push((key, SyntaxShape::Any));
            break;
        } else {
            idx += 1;
        }

        let maybe_colon = working_set.get_span_contents(tokens[idx].span);
        match maybe_colon {
            b":" => {
                if idx + 1 == tokens.len() {
                    working_set.error(ParseError::Expected("type after colon", tokens[idx].span));
                    break;
                } else {
                    idx += 1;
                }
            }
            // a key provided without a type
            b"," => {
                idx += 1;
                sig.push((key, SyntaxShape::Any));
                continue;
            }
            // a key provided without a type
            _ => {
                sig.push((key, SyntaxShape::Any));
                continue;
            }
        }

        let shape_bytes = working_set.get_span_contents(tokens[idx].span).to_vec();
        let shape = parse_shape_name(working_set, &shape_bytes, tokens[idx].span);
        sig.push((key, shape));
        idx += 1;
    }

    sig.into()
}

fn parse_type_params(
    working_set: &mut StateWorkingSet,
    Spanned { item: source, span }: Spanned<&[u8]>,
) -> Vec<SyntaxShape> {
    let (tokens, err) = lex_signature(source, span.start, &[b'\n', b'\r'], &[b':', b','], true);

    if let Some(err) = err {
        working_set.error(err);
        return Vec::new();
    }

    let mut sig = vec![];
    let mut idx = 0;

    while idx < tokens.len() {
        if working_set
            .get_span_contents(tokens[idx].span)
            .starts_with(b",")
        {
            idx += 1;
            continue;
        }

        let shape_bytes = working_set.get_span_contents(tokens[idx].span).to_vec();
        let shape = parse_shape_name(working_set, &shape_bytes, tokens[idx].span);
        sig.push(shape);
        idx += 1;
    }

    sig
}
