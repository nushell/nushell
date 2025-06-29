#![allow(clippy::byte_char_slices)]

use crate::{TokenContents, lex::lex_signature, parser::parse_value, trim_quotes};
use nu_protocol::{
    IntoSpanned, ParseError, Span, Spanned, SyntaxShape, Type, engine::StateWorkingSet,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShapeDescriptorUse {
    /// Used in an argument position allowing the addition of custom completion
    Argument,
    /// Used to define the type of a variable or input/output types
    Type,
}

/// equivalent to [`parse_shape_name`] with [`ShapeDescriptorUse::Type`] converting the
/// [`SyntaxShape`] to its [`Type`]
pub fn parse_type(working_set: &mut StateWorkingSet, bytes: &[u8], span: Span) -> Type {
    parse_shape_name(working_set, bytes, span, ShapeDescriptorUse::Type).to_type()
}

/// Parse the literals of [`Type`]-like [`SyntaxShape`]s including inner types.
/// Also handles the specification of custom completions with `type@completer`.
///
/// Restrict the parsing with `use_loc`
/// Used in:
/// - [`ShapeDescriptorUse::Argument`]
///   - `: ` argument type (+completer) positions in signatures
/// - [`ShapeDescriptorUse::Type`]
///   - `type->type` input/output type pairs
///   - `let name: type` variable type infos
///
/// NOTE: Does not provide a mapping to every [`SyntaxShape`]
pub fn parse_shape_name(
    working_set: &mut StateWorkingSet,
    bytes: &[u8],
    span: Span,
    use_loc: ShapeDescriptorUse,
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
            parse_generic_shape(working_set, bytes, span, use_loc)
        }
        _ => {
            if bytes.contains(&b'@') {
                let mut split = bytes.splitn(2, |b| b == &b'@');

                let shape_name = split
                    .next()
                    .expect("If `bytes` contains `@` splitn returns 2 slices");
                let shape_span = Span::new(span.start, span.start + shape_name.len());
                let shape = parse_shape_name(working_set, shape_name, shape_span, use_loc);
                if use_loc != ShapeDescriptorUse::Argument {
                    let illegal_span = Span::new(span.start + shape_name.len(), span.end);
                    working_set.error(ParseError::LabeledError(
                        "Unexpected custom completer in type spec".into(),
                        "Type specifications do not support custom completers".into(),
                        illegal_span,
                    ));
                    return shape;
                }

                let cmd_span = Span::new(span.start + shape_name.len() + 1, span.end);
                let cmd_name = split
                    .next()
                    .expect("If `bytes` contains `@` splitn returns 2 slices");

                let cmd_name = trim_quotes(cmd_name);
                if cmd_name.is_empty() {
                    working_set.error(ParseError::Expected(
                        "the command name of a completion function",
                        cmd_span,
                    ));
                    return shape;
                }

                if let Some(decl_id) = working_set.find_decl(cmd_name) {
                    SyntaxShape::CompleterWrapper(Box::new(shape), decl_id)
                } else {
                    working_set.error(ParseError::UnknownCommand(cmd_span));
                    shape
                }
            } else {
                //TODO: Handle error case for unknown shapes
                working_set.error(ParseError::UnknownType(span));
                SyntaxShape::Any
            }
        }
    }
}

fn parse_generic_shape(
    working_set: &mut StateWorkingSet<'_>,
    bytes: &[u8],
    span: Span,
    use_loc: ShapeDescriptorUse,
) -> SyntaxShape {
    let (type_name, type_params) = split_generic_params(working_set, bytes, span);
    match type_name {
        b"oneof" => SyntaxShape::OneOf(match type_params {
            Some(params) => parse_type_params(working_set, params, use_loc),
            None => vec![],
        }),
        b"list" => SyntaxShape::List(Box::new(match type_params {
            Some(params) => {
                let mut parsed_params = parse_type_params(working_set, params, use_loc);
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
            Some(params) => parse_named_type_params(working_set, params, use_loc),
            None => vec![],
        }),
        b"table" => SyntaxShape::Table(match type_params {
            Some(params) => parse_named_type_params(working_set, params, use_loc),
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
    use_loc: ShapeDescriptorUse,
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
        let shape = parse_shape_name(working_set, &shape_bytes, tokens[idx].span, use_loc);
        sig.push((key, shape));
        idx += 1;
    }

    sig
}

fn parse_type_params(
    working_set: &mut StateWorkingSet,
    Spanned { item: source, span }: Spanned<&[u8]>,
    use_loc: ShapeDescriptorUse,
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
        let shape = parse_shape_name(working_set, &shape_bytes, tokens[idx].span, use_loc);
        sig.push(shape);
        idx += 1;
    }

    sig
}
