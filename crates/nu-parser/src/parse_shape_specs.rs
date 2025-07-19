#![allow(clippy::byte_char_slices)]

use crate::{TokenContents, lex::lex_signature, parser::parse_value, trim_quotes};
use nu_protocol::{
    DeclId, IntoSpanned, ParseError, Span, Spanned, SyntaxShape, Type, engine::StateWorkingSet,
};

/// [`parse_shape_name`] then convert to Type
pub fn parse_type(working_set: &mut StateWorkingSet, bytes: &[u8], span: Span) -> Type {
    parse_shape_name(working_set, bytes, span).to_type()
}

/// Parse the literals of [`Type`]-like [`SyntaxShape`]s including inner types.
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
        b"float" => SyntaxShape::Float,
        b"filesize" => SyntaxShape::Filesize,
        b"glob" => SyntaxShape::GlobPattern,
        b"int" => SyntaxShape::Int,
        b"nothing" => SyntaxShape::Nothing,
        b"number" => SyntaxShape::Number,
        b"path" => SyntaxShape::Filepath,
        b"range" => SyntaxShape::Range,
        b"string" => SyntaxShape::String,
        _ if bytes.starts_with(b"oneof")
            || bytes.starts_with(b"list")
            || bytes.starts_with(b"record")
            || bytes.starts_with(b"table") =>
        {
            parse_generic_shape(working_set, bytes, span)
        }
        _ => {
            if bytes.contains(&b'@') {
                working_set.error(ParseError::LabeledError(
                    "Unexpected custom completer in type spec".into(),
                    "Type specifications do not support custom completers".into(),
                    span,
                ));
            }
            //TODO: Handle error case for unknown shapes
            working_set.error(ParseError::UnknownType(span));
            SyntaxShape::Any
        }
    }
}

/// Handles the specification of custom completions with `type@completer`.
pub fn parse_completer(
    working_set: &mut StateWorkingSet,
    bytes: &[u8],
    span: Span,
) -> Option<DeclId> {
    let cmd_name = trim_quotes(bytes);
    if cmd_name.is_empty() {
        working_set.error(ParseError::Expected(
            "the command name of a completion function",
            span,
        ));
        return None;
    }
    working_set.find_decl(cmd_name)
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
        b"record" => SyntaxShape::Record(match type_params {
            Some(params) => parse_named_type_params(working_set, params),
            None => vec![],
        }),
        b"table" => SyntaxShape::Table(match type_params {
            Some(params) => parse_named_type_params(working_set, params),
            None => vec![],
        }),
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
    let n = bytes.iter().position(|&c| c == b'<');
    let (open_delim_pos, close_delim) = match n.and_then(|n| Some((n, bytes.get(n)?))) {
        Some((n, b'<')) => (n, b'>'),
        _ => return (bytes, None),
    };

    let type_name = &bytes[..(open_delim_pos)];
    let params = &bytes[(open_delim_pos + 1)..];

    let start = span.start + type_name.len() + 1;

    if params.ends_with(&[close_delim]) {
        let end = span.end - 1;
        (
            type_name,
            Some((&params[..(params.len() - 1)]).into_spanned(Span::new(start, end))),
        )
    } else if let Some(close_delim_pos) = params.iter().position(|it| it == &close_delim) {
        let span = Span::new(span.start + close_delim_pos, span.end);

        working_set.error(ParseError::LabeledError(
            "Extra characters in the parameter name".into(),
            "extra characters".into(),
            span,
        ));

        (bytes, None)
    } else {
        working_set.error(ParseError::Unclosed((close_delim as char).into(), span));
        (bytes, None)
    }
}

fn parse_named_type_params(
    working_set: &mut StateWorkingSet,
    Spanned { item: source, span }: Spanned<&[u8]>,
) -> Vec<(String, SyntaxShape)> {
    let (tokens, err) = lex_signature(source, span.start, &[b'\n', b'\r'], &[b':', b','], true);

    if let Some(err) = err {
        working_set.error(err);
        return Vec::new();
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
            return Vec::new();
        };

        if working_set
            .get_span_contents(tokens[idx].span)
            .starts_with(b",")
        {
            idx += 1;
            continue;
        }

        let Some(key) =
            parse_value(working_set, tokens[idx].span, &SyntaxShape::String).as_string()
        else {
            working_set.error(key_error(tokens[idx].span));
            return Vec::new();
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

    sig
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
