//! Types and shapes read from their text, as nu-parser's `parse_shape_specs.rs` reads them: a
//! parameter's type (`parse_shape_name`), a variable's (`parse_type`) and a parameter's
//! completer (`parse_completer`). As there, the text read and the span reported may differ (a
//! variable's type is read from its whole annotation, reported at its first item), and a type's
//! parameters are lexed from the text and read back from the working set at the spans lexed.

use std::borrow::Cow;

use nu_protocol::{
    CollectionColumns, Completion, ParseError, ShellError, Span, SyntaxShape, Type, Value,
    eval_const::eval_constant,
};
use nu_utils::NuCow;

use super::Lower;
use crate::winnow::syntax::Span as WSpan;
use crate::winnow::syntax::lex::{GroupEnds, LexOptions, Token, TokenContents, lex_recovering};

/// The part of a type before its type parameters: `list` for `list<int>` (nu's `type_name`).
fn type_name(text: &str) -> &str {
    text.find('<').map_or(text, |open| &text[..open])
}

impl Lower<'_, '_, '_> {
    /// The type `text` names, reported at `span` (`parse_type`).
    pub(super) fn type_text(&mut self, text: &str, span: Span) -> Type {
        self.shape_name(text, span).to_type()
    }

    /// The shape `text` names, reported at `span` (`parse_shape_name`): a built-in type, a
    /// generic one with its parameters (`list<int>`, `record<a: int>`, `oneof<a, b>`), a type
    /// alias in scope or a plugin's custom type; anything else reported, as `any`.
    pub(super) fn shape_name(&mut self, text: &str, span: Span) -> SyntaxShape {
        match text {
            "any" => SyntaxShape::Any,
            "binary" => SyntaxShape::Binary,
            "block" => {
                self.working_set.error(ParseError::LabeledErrorWithHelp {
                    error: "Blocks are not support as first-class values".into(),
                    label: "blocks are not supported as values".into(),
                    help: "Use 'closure' instead of 'block'".into(),
                    span,
                });
                SyntaxShape::Any
            }
            "bool" => SyntaxShape::Boolean,
            "cell-path" => SyntaxShape::CellPath,
            "closure" => SyntaxShape::Closure(None),
            "datetime" => SyntaxShape::DateTime,
            "directory" => SyntaxShape::Directory,
            "duration" => SyntaxShape::Duration,
            "error" => SyntaxShape::Error,
            "external_arg" => SyntaxShape::ExternalArgument,
            "float" => SyntaxShape::Float,
            "filesize" => SyntaxShape::Filesize,
            "glob" => SyntaxShape::GlobPattern,
            "int" => SyntaxShape::Int,
            "nothing" => SyntaxShape::Nothing,
            "number" => SyntaxShape::Number,
            "path" => SyntaxShape::Filepath,
            "range" => SyntaxShape::Range,
            "string" => SyntaxShape::String,
            _ if matches!(type_name(text), "oneof" | "list" | "record" | "table") => {
                self.generic_shape(text, span)
            }
            _ if text.contains('@') => {
                self.working_set.error(ParseError::LabeledError(
                    "Unexpected custom completer in type spec".into(),
                    "Type specifications do not support custom completers".into(),
                    span,
                ));
                self.working_set.error(ParseError::UnknownType(span));
                SyntaxShape::Any
            }
            _ => self.type_alias_shape(text, span),
        }
    }

    /// What the type alias `text` expands to (`expand_type_alias`), a plugin's custom type, or,
    /// reported, why it names no type: used before its definition, inside it, or unknown.
    fn type_alias_shape(&mut self, text: &str, span: Span) -> SyntaxShape {
        let name = type_name(text);
        if let Some(type_alias_id) = self.working_set.find_type_alias(name.as_bytes()) {
            if name.len() < text.len() {
                self.working_set.error(ParseError::TypeAliasWithParams(
                    name.to_string(),
                    Span::new(span.start + name.len(), span.end),
                ));
            }
            return self.working_set.get_type_alias(type_alias_id).shape.clone();
        }
        if name.len() == text.len() && self.working_set.find_custom_type(name.as_bytes()) {
            return SyntaxShape::Custom(name.into());
        }
        let definition = self
            .working_set
            .find_type_predecls(name.as_bytes())
            .find(|definition| definition.end > span.start);
        let error = match definition {
            Some(definition) if definition.contains_span(span) => {
                ParseError::TypeAliasSelfReference(name.to_string(), span)
            }
            Some(definition) => {
                ParseError::TypeAliasUsedBeforeDefinition(name.to_string(), span, definition)
            }
            None => ParseError::UnknownType(span),
        };
        self.working_set.error(error);
        SyntaxShape::Any
    }

    /// A generic type with its parameters (`parse_generic_shape`).
    fn generic_shape(&mut self, text: &str, span: Span) -> SyntaxShape {
        let (name, params) = self.split_generic_params(text, span);
        match name {
            "oneof" => SyntaxShape::OneOf(match params {
                Some((params, params_span)) => self.type_params(params, params_span),
                None => Vec::new(),
            }),
            "list" => SyntaxShape::List(Box::new(match params {
                Some((params, params_span)) => {
                    let mut shapes = self.type_params(params, params_span);
                    if shapes.len() > 1 {
                        self.working_set.error(ParseError::LabeledError(
                            "expected a single type parameter".into(),
                            "only one parameter allowed".into(),
                            params_span,
                        ));
                        SyntaxShape::Any
                    } else {
                        shapes.pop().unwrap_or(SyntaxShape::Any)
                    }
                }
                None => SyntaxShape::Any,
            })),
            "record" => SyntaxShape::Record(
                params
                    .map(|(params, params_span)| self.named_type_params(params, params_span))
                    .unwrap_or_default(),
            ),
            "table" => SyntaxShape::Table(
                params
                    .map(|(params, params_span)| self.named_type_params(params, params_span))
                    .unwrap_or_default(),
            ),
            _ => {
                self.working_set.error(ParseError::UnknownType(span));
                SyntaxShape::Any
            }
        }
    }

    /// A generic type's name and the text of its parameters between `<` and `>`, with their
    /// span (`split_generic_params`); text after the `>`, or no `>`, reported.
    fn split_generic_params<'t>(
        &mut self,
        text: &'t str,
        span: Span,
    ) -> (&'t str, Option<(&'t str, Span)>) {
        let Some(open) = text.find('<') else {
            return (text, None);
        };
        let name = &text[..open];
        let params = &text[open + 1..];
        let start = span.start + name.len() + 1;
        if let Some(inner) = params.strip_suffix('>') {
            return (name, Some((inner, Span::new(start, span.end - 1))));
        }
        match params.find('>') {
            Some(close) => {
                self.working_set.error(ParseError::LabeledError(
                    "Extra characters in the parameter name".into(),
                    "extra characters".into(),
                    Span::new(span.start + close, span.end),
                ));
            }
            None => {
                let open = ParseError::opener_span(span, 1);
                self.working_set
                    .error(ParseError::unclosed(">", open, span));
            }
        }
        (text, None)
    }

    /// The tokens of a type's parameters `text`, whose first byte is at `span`'s start, lexed as
    /// nu's `lex_signature` lexes them there (`:` and `,` apart, comments dropped); the first
    /// lexer error reported, with no tokens.
    fn type_param_tokens(&mut self, text: &str, span: Span) -> Option<Vec<Token>> {
        let base = span.start.saturating_sub(self.offset);
        let local = WSpan::new(base, base + text.len());
        let lexed = lex_recovering(
            text,
            base,
            LexOptions::TYPE_PARAMS,
            usize::MAX,
            &mut GroupEnds::new(local),
        )
        .ok()?;
        if let Some((_, error)) = lexed.errors.first() {
            let error = self.lex_parse_error(error, base);
            self.working_set.error(error);
            return None;
        }
        Some(
            lexed
                .tokens
                .into_iter()
                .filter(|token| {
                    !matches!(token.contents, TokenContents::Eof | TokenContents::Comment)
                })
                .collect(),
        )
    }

    /// The text of the working set at a lexed token's span.
    fn token_text(&self, token: &Token) -> String {
        String::from_utf8_lossy(self.working_set.get_span_contents(self.span(token.span))).into()
    }

    /// The shapes of `oneof<...>` or `list<...>`'s parameters (`parse_type_params`).
    fn type_params(&mut self, text: &str, span: Span) -> Vec<SyntaxShape> {
        let Some(tokens) = self.type_param_tokens(text, span) else {
            return Vec::new();
        };
        let mut shapes = Vec::new();
        for token in &tokens {
            let item = self.token_text(token);
            if item.starts_with(',') {
                continue;
            }
            let item_span = self.span(token.span);
            shapes.push(self.shape_name(&item, item_span));
        }
        shapes
    }

    /// The named columns of `record<...>` or `table<...>` (`parse_named_type_params`): a key,
    /// a `:` and a type, or a key alone (`any`); a key that is no string is reported, with no
    /// columns.
    fn named_type_params(&mut self, text: &str, span: Span) -> CollectionColumns<SyntaxShape> {
        let Some(tokens) = self.type_param_tokens(text, span) else {
            return CollectionColumns::default();
        };
        let key_error = |span| {
            ParseError::LabeledError(
                "annotation key not string".into(),
                "must be a string".into(),
                span,
            )
        };
        let mut columns = Vec::new();
        let mut index = 0;
        while index < tokens.len() {
            let token_span = self.span(tokens[index].span);
            if tokens[index].contents != TokenContents::Item {
                self.working_set.error(key_error(token_span));
                return CollectionColumns::default();
            }
            if self.token_text(&tokens[index]).starts_with(',') {
                index += 1;
                continue;
            }
            let key = match self.value_text(tokens[index].span, &SyntaxShape::String) {
                Ok(key) => key.as_string(),
                Err(_) => None,
            };
            let Some(key) = key else {
                self.working_set.error(key_error(token_span));
                return CollectionColumns::default();
            };
            if index + 1 == tokens.len() {
                columns.push((key, SyntaxShape::Any));
                break;
            }
            index += 1;
            match self.token_text(&tokens[index]).as_str() {
                ":" => {
                    if index + 1 == tokens.len() {
                        let colon = self.span(tokens[index].span);
                        self.working_set
                            .error(ParseError::Expected("type after colon", colon));
                        break;
                    }
                    index += 1;
                }
                "," => {
                    index += 1;
                    columns.push((key, SyntaxShape::Any));
                    continue;
                }
                _ => {
                    columns.push((key, SyntaxShape::Any));
                    continue;
                }
            }
            let type_text = self.token_text(&tokens[index]);
            let type_span = self.span(tokens[index].span);
            let shape = self.shape_name(&type_text, type_span);
            columns.push((key, shape));
            index += 1;
        }
        columns.into()
    }

    /// A parameter's completer after its `@` (`parse_completer`): a command's name, or a list
    /// of suggestions, each a string or a record with a `value`; anything else reported.
    pub(super) fn completer_text(&mut self, span: WSpan) -> Option<Completion> {
        let completer_span = self.span(span);
        let errors = self.working_set.parse_errors.len();
        let shape = SyntaxShape::OneOf(vec![
            SyntaxShape::List(Box::new(SyntaxShape::String)),
            SyntaxShape::String,
        ]);
        let expression = self.value_text(span, &shape).ok()?;
        if self.working_set.parse_errors.len() > errors {
            return None;
        }
        let value = match eval_constant(self.working_set, &expression) {
            Ok(value) => value,
            Err(error) => {
                let error = error.wrap(self.working_set, completer_span);
                self.working_set.error(error);
                return None;
            }
        };
        let completion = match value {
            Value::List { vals, .. } => vals
                .into_iter()
                .map(|value| {
                    let span = value.span();
                    match value {
                        Value::Record { val, .. } => val
                            .get("value")
                            .ok_or(ShellError::CantFindColumn {
                                col_name: "value".into(),
                                span: None,
                                src_span: span,
                            })
                            .and_then(Value::coerce_str)
                            .map(Cow::into_owned),
                        value => value.coerce_into_string(),
                    }
                })
                .collect::<Result<Vec<_>, ShellError>>()
                .map_err(|error| error.wrap(self.working_set, completer_span))
                .map(|values| Completion::List(NuCow::Owned(values))),
            Value::String { val, .. } => self
                .working_set
                .find_decl(val.as_bytes())
                .map(Completion::Command)
                .ok_or(ParseError::UnknownCommand(completer_span)),
            value => Err(ParseError::OperatorUnsupportedType {
                op: "parameter completer",
                unsupported: value.get_type(),
                op_span: Span::new(completer_span.start - 1, completer_span.start),
                unsupported_span: completer_span,
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
            Err(error) => {
                self.working_set.error(error);
                None
            }
        }
    }
}
