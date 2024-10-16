use crate::{lex::lex_signature, parser::parse_value, trim_quotes, TokenContents};
use nu_protocol::{engine::StateWorkingSet, ParseError, Span, SyntaxShape, Type};

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
    let result = match bytes {
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
        _ if bytes.starts_with(b"list") => parse_list_shape(working_set, bytes, span, use_loc),
        b"nothing" => SyntaxShape::Nothing,
        b"number" => SyntaxShape::Number,
        b"path" => SyntaxShape::Filepath,
        b"range" => SyntaxShape::Range,
        _ if bytes.starts_with(b"record") => {
            parse_collection_shape(working_set, bytes, span, use_loc)
        }
        b"string" => SyntaxShape::String,
        _ if bytes.starts_with(b"table") => {
            parse_collection_shape(working_set, bytes, span, use_loc)
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
                    return SyntaxShape::CompleterWrapper(Box::new(shape), decl_id);
                } else {
                    working_set.error(ParseError::UnknownCommand(cmd_span));
                    return shape;
                }
            } else {
                //TODO: Handle error case for unknown shapes
                working_set.error(ParseError::UnknownType(span));
                return SyntaxShape::Any;
            }
        }
    };

    result
}

fn parse_collection_shape(
    working_set: &mut StateWorkingSet,
    bytes: &[u8],
    span: Span,
    use_loc: ShapeDescriptorUse,
) -> SyntaxShape {
    assert!(bytes.starts_with(b"record") || bytes.starts_with(b"table"));
    let is_table = bytes.starts_with(b"table");

    let name = if is_table { "table" } else { "record" };
    let prefix = (if is_table { "table<" } else { "record<" }).as_bytes();
    let prefix_len = prefix.len();
    let mk_shape = |ty| -> SyntaxShape {
        if is_table {
            SyntaxShape::Table(ty)
        } else {
            SyntaxShape::Record(ty)
        }
    };

    if bytes == name.as_bytes() {
        mk_shape(vec![])
    } else if bytes.starts_with(prefix) {
        let Some(inner_span) = prepare_inner_span(working_set, bytes, span, prefix_len) else {
            return SyntaxShape::Any;
        };

        // record<> or table<>
        if inner_span.end - inner_span.start == 0 {
            return mk_shape(vec![]);
        }
        let source = working_set.get_span_contents(inner_span);
        let (tokens, err) = lex_signature(
            source,
            inner_span.start,
            &[b'\n', b'\r'],
            &[b':', b','],
            true,
        );

        if let Some(err) = err {
            working_set.error(err);
            // lexer errors cause issues with span overflows
            return mk_shape(vec![]);
        }

        let mut sig = vec![];
        let mut idx = 0;

        let key_error = |span| {
            ParseError::LabeledError(
                format!("`{name}` type annotations key not string"),
                "must be a string".into(),
                span,
            )
        };

        while idx < tokens.len() {
            let TokenContents::Item = tokens[idx].contents else {
                working_set.error(key_error(tokens[idx].span));
                return mk_shape(vec![]);
            };

            let key_bytes = working_set.get_span_contents(tokens[idx].span).to_vec();
            if key_bytes.first().copied() == Some(b',') {
                idx += 1;
                continue;
            }

            let Some(key) =
                parse_value(working_set, tokens[idx].span, &SyntaxShape::String).as_string()
            else {
                working_set.error(key_error(tokens[idx].span));
                return mk_shape(vec![]);
            };

            // we want to allow such an annotation
            // `record<name>` where the user leaves out the type
            if idx + 1 == tokens.len() {
                sig.push((key, SyntaxShape::Any));
                break;
            } else {
                idx += 1;
            }

            let maybe_colon = working_set.get_span_contents(tokens[idx].span).to_vec();
            match maybe_colon.as_slice() {
                b":" => {
                    if idx + 1 == tokens.len() {
                        working_set
                            .error(ParseError::Expected("type after colon", tokens[idx].span));
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

        mk_shape(sig)
    } else {
        working_set.error(ParseError::UnknownType(span));

        SyntaxShape::Any
    }
}

fn parse_list_shape(
    working_set: &mut StateWorkingSet,
    bytes: &[u8],
    span: Span,
    use_loc: ShapeDescriptorUse,
) -> SyntaxShape {
    assert!(bytes.starts_with(b"list"));

    if bytes == b"list" {
        SyntaxShape::List(Box::new(SyntaxShape::Any))
    } else if bytes.starts_with(b"list<") {
        let Some(inner_span) = prepare_inner_span(working_set, bytes, span, 5) else {
            return SyntaxShape::Any;
        };

        let inner_text = String::from_utf8_lossy(working_set.get_span_contents(inner_span));
        // remove any extra whitespace, for example `list< string >` becomes `list<string>`
        let inner_bytes = inner_text.trim().as_bytes().to_vec();

        // list<>
        if inner_bytes.is_empty() {
            SyntaxShape::List(Box::new(SyntaxShape::Any))
        } else {
            let inner_sig = parse_shape_name(working_set, &inner_bytes, inner_span, use_loc);

            SyntaxShape::List(Box::new(inner_sig))
        }
    } else {
        working_set.error(ParseError::UnknownType(span));

        SyntaxShape::List(Box::new(SyntaxShape::Any))
    }
}

fn prepare_inner_span(
    working_set: &mut StateWorkingSet,
    bytes: &[u8],
    span: Span,
    prefix_len: usize,
) -> Option<Span> {
    let start = span.start + prefix_len;

    if bytes.ends_with(b">") {
        let end = span.end - 1;
        Some(Span::new(start, end))
    } else if bytes.contains(&b'>') {
        let angle_start = bytes.split(|it| it == &b'>').collect::<Vec<_>>()[0].len() + 1;
        let span = Span::new(span.start + angle_start, span.end);

        working_set.error(ParseError::LabeledError(
            "Extra characters in the parameter name".into(),
            "extra characters".into(),
            span,
        ));

        None
    } else {
        working_set.error(ParseError::Unclosed(">".into(), span));
        None
    }
}
