//! Lists and tables read from their text, as nu-parser's `parse_list_expression` and
//! `parse_table_expression` read them: for a `[...]` item the winnow parser went on past an
//! error in, and a table the lowering hands back for nu's errors. The inside is lexed as nu lexes
//! list items and lite-parsed ([`Lower::lite_parse`]); each item is read with the element shape
//! from its text.

use std::collections::{HashMap, hash_map::Entry};

use nu_protocol::{
    IntoSpanned, ParseError, Span, SyntaxShape, Type, TypeSet,
    ast::{Expr, Expression, ListItem, Table},
};

use super::{Lower, Lowered, Unlowered};
use crate::parse_expressions::table_type;
use crate::parse_helpers::extract_spread_list;
use crate::winnow::syntax::lex::{GroupEnds, LexOptions, Token, TokenContents, lex_recovering};
use crate::winnow::syntax::{Span as WSpan, ast as w};

/// What the items of a list are (nu's `ListItems`).
#[derive(Clone, Copy)]
pub(super) enum ListItems<'a> {
    /// Values of a shape.
    Values(&'a SyntaxShape),
    /// The column names of a table's header row, each read as a record key.
    Names,
}

impl<'s> Lower<'_, '_, 's> {
    /// The tokens of a `[...]`'s inside covering `inner`, lexed as nu lexes list items (commas
    /// and newlines are whitespace, comments dropped); the first lexer error reported.
    fn list_tokens(&mut self, inner: WSpan) -> Vec<Token> {
        let lexed = lex_recovering(
            self.text(inner),
            inner.start,
            LexOptions::LIST,
            usize::MAX,
            &mut GroupEnds::new(inner),
        );
        let Ok(lexed) = lexed else {
            return Vec::new();
        };
        if let Some((_, error)) = lexed.errors.first() {
            let error = self.lex_parse_error(error, inner.start);
            self.working_set.error(error);
        }
        lexed
            .tokens
            .into_iter()
            .filter(|token| !matches!(token.contents, TokenContents::Eof | TokenContents::Comment))
            .collect()
    }

    /// The inside of the `[...]` covering `span`, and whether the `]` closes it.
    fn list_inside(&self, span: WSpan) -> (WSpan, bool) {
        let text = self.text(span);
        let start = span.start + usize::from(text.starts_with('['));
        let closed = text.ends_with(']');
        let end = if closed { span.end - 1 } else { span.end };
        (WSpan::new(start, end.max(start)), closed)
    }

    /// A list read from the text covering `span` (`parse_list_expression`), from `tokens` when
    /// the caller lexed its inside (and reported what that found): a `;` reported, the inside
    /// lite-parsed, and the items of its first pipeline read with the element shape (a spread
    /// as a list of it). Its type is the union of its items'.
    pub(super) fn list_text(
        &mut self,
        span: WSpan,
        items: ListItems<'_>,
        tokens: Option<Vec<Token>>,
    ) -> Lowered<Expression> {
        let full = self.span(span);
        let tokens = match tokens {
            Some(tokens) => tokens,
            None => {
                let (inside, closed) = self.list_inside(span);
                if !closed {
                    let open = ParseError::opener_span(full, 1);
                    self.working_set.error(ParseError::unclosed(
                        "]",
                        open,
                        Span::new(full.end, full.end),
                    ));
                }
                self.list_tokens(inside)
            }
        };
        if let Some(semicolon) = tokens
            .iter()
            .find(|token| token.contents == TokenContents::Semicolon)
        {
            let span = self.span(semicolon.span);
            self.working_set.error(ParseError::LabeledErrorWithHelp {
                error: "Unexpected semicolon in list".into(),
                label: "not a valid list separator".into(),
                help: "Use commas or whitespace to separate list items.".into(),
                span,
            });
        }
        let (block, error) = self.lite_parse(&tokens);
        if let Some(error) = error {
            self.working_set.error(error);
        }
        let mut out = Vec::new();
        let mut contained: Option<Type> = None;
        if let Some(pipeline) = block.pipelines.into_iter().next() {
            for command in pipeline {
                for part in command.parts {
                    let bytes = self.working_set.get_span_contents(part);
                    let spread =
                        extract_spread_list(bytes.into_spanned(part)).map(|spread| spread.span);
                    let (item, ty) = match spread {
                        Some(list) => {
                            let element = match items {
                                ListItems::Values(shape) => shape.clone(),
                                ListItems::Names => SyntaxShape::String,
                            };
                            let shape = SyntaxShape::List(Box::new(element));
                            let value = self.value_text(self.local(list), &shape)?;
                            let ty = match &value.ty {
                                Type::List(element) => *element.clone(),
                                _ => Type::Any,
                            };
                            let dots = Span::new(part.start, part.start + 3);
                            (ListItem::Spread(dots, value), ty)
                        }
                        None => {
                            let local = self.local(part);
                            let value = match items {
                                ListItems::Values(shape) => self.value_text(local, shape)?,
                                ListItems::Names => {
                                    let name = w::Expression::new(w::Expr::Garbage, local);
                                    self.field_name(&name)?
                                }
                            };
                            let ty = value.ty.clone();
                            (ListItem::Item(value), ty)
                        }
                    };
                    contained = Some(match contained {
                        Some(contained) => contained.union(ty),
                        None => ty,
                    });
                    out.push(item);
                }
            }
        }
        let ty = Type::List(Box::new(contained.unwrap_or(Type::Any)));
        Ok(self.node(Expr::List(out), full, ty))
    }

    /// One row of a table read from its text (`parse_table_row`): its values, or the span of a
    /// spread in it, which a row cannot have.
    fn table_row_text(
        &mut self,
        span: WSpan,
        items: ListItems<'_>,
    ) -> Lowered<Result<(Vec<Expression>, Span), Span>> {
        let list = self.list_text(span, items, None)?;
        let Expression {
            expr: Expr::List(list),
            span,
            ..
        } = list
        else {
            return Err(Unlowered::Error);
        };
        Ok(list
            .into_iter()
            .map(|item| match item {
                ListItem::Item(value) => Ok(value),
                ListItem::Spread(_, spread) => Err(spread.span),
            })
            .collect::<Result<_, _>>()
            .map(|values| (values, span)))
    }

    /// A list or a table read from the text covering `span` (`parse_table_expression`): a table
    /// when its inside starts with a `[...]` row and a `;`, its header's names read as record
    /// keys and each later row a list of values, else a list of `element`.
    pub(super) fn table_text(&mut self, span: WSpan, element: &SyntaxShape) -> Lowered<Expression> {
        let full = self.span(span);
        let (inside, closed) = self.list_inside(span);
        if !closed {
            let open = ParseError::opener_span(full, 1);
            self.working_set.error(ParseError::unclosed(
                "]",
                open,
                Span::new(full.end, full.end),
            ));
        }
        let tokens = self.list_tokens(inside);
        let items = ListItems::Values(element);
        let is_table = matches!(
            tokens.as_slice(),
            [first, second, ..] if self.text(first.span).starts_with('[')
                && second.contents == TokenContents::Semicolon
        );
        if !is_table {
            return self.list_text(span, items, Some(tokens));
        }
        let (first, second, rest) = (tokens[0], tokens[1], &tokens[2..]);
        let head = self.table_row_text(first.span, ListItems::Names)?;
        let errors = self.working_set.parse_errors.len();
        let spread_error = |span: Span| {
            ParseError::LabeledError(
                "Cannot spread in a table row".into(),
                "invalid spread here".into(),
                span,
            )
        };
        let (head, rows) = match head {
            Ok((head, _)) => {
                let mut names: HashMap<String, Span> = HashMap::new();
                for column in &head {
                    let (Expr::String(name) | Expr::RawString(name)) = &column.expr else {
                        continue;
                    };
                    match names.entry(name.clone()) {
                        Entry::Occupied(first) => {
                            let error = ParseError::ColumnDefinedTwice(
                                name.clone(),
                                column.span,
                                *first.get(),
                            );
                            self.working_set.error(error);
                        }
                        Entry::Vacant(entry) => {
                            entry.insert(column.span);
                        }
                    }
                }
                if rest.is_empty() {
                    let end = self.span(second.span).end;
                    self.working_set
                        .error(ParseError::Expected("table row", Span::new(end, end)));
                }
                let mut rows = Vec::new();
                for item in rest {
                    let item_span = self.span(item.span);
                    match self.text(item.span) {
                        "," => {}
                        text if !text.starts_with('[') => {
                            self.working_set.error(ParseError::LabeledErrorWithHelp {
                                error: "Table item not list".into(),
                                label: "not a list".into(),
                                span: item_span,
                                help: "All table items must be lists".into(),
                            });
                        }
                        _ => match self
                            .table_row_text(item.span, ListItems::Values(&SyntaxShape::Any))?
                        {
                            Ok((row, row_span)) => {
                                if row.len() < head.len() {
                                    self.working_set
                                        .error(ParseError::MissingColumns(head.len(), row_span));
                                } else if row.len() > head.len() {
                                    let extra = Span::new(row[head.len()].span.start, row_span.end);
                                    self.working_set
                                        .error(ParseError::ExtraColumns(head.len(), extra));
                                }
                                rows.push(row);
                            }
                            Err(spread) => self.working_set.error(spread_error(spread)),
                        },
                    }
                }
                (head, rows)
            }
            Err(spread) => {
                self.working_set.error(spread_error(spread));
                (Vec::new(), Vec::new())
            }
        };
        let ty = match self.working_set.parse_errors.len() == errors {
            true => {
                let (ty, type_errors) = table_type(&head, &rows);
                self.working_set.parse_errors.extend(type_errors);
                ty
            }
            false => Type::table(),
        };
        let table = Table {
            columns: head.into(),
            rows: rows.into_iter().map(Into::into).collect(),
        };
        Ok(self.node(Expr::Table(table), full, ty))
    }
}
