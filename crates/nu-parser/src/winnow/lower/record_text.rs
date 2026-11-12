//! Records and `{...}` items read from their text, as nu-parser's `parse_record` and
//! `parse_brace_expr` read them: for a `{...}` item the winnow parser went on past an error in,
//! a record head of a cell path, and a `{` external argument. A closure, a block or a match
//! block is still nu's reader's, until those are read here.

use std::collections::{HashMap, hash_map::Entry};

use nu_protocol::{
    IntoSpanned, ParseError, Span, SyntaxShape, Type,
    ast::{Expr, Expression, RecordItem},
};

use super::{Lower, Lowered};
use crate::parse_expressions::check_record_key_or_value;
use crate::parse_helpers::extract_spread_record;
use crate::winnow::syntax::lex::{
    GroupEnds, LexError, LexOptions, Token, TokenContents, lex_recovering,
};
use crate::winnow::syntax::{Span as WSpan, ast as w};

/// The help for a token where a record key goes that is none (nu's
/// `record_key_position_help`).
fn record_key_position_help(found: &str) -> String {
    match found {
        ";" => "Records use newlines or commas between fields, not `;`. \
                `;` separates pipelines/statements in Nushell."
            .into(),
        "," => "Unexpected comma here. Put commas between fields as `key: value, key2: value2`, \
                or use a newline instead."
            .into(),
        "|" | "||" => "Unexpected pipe in a record. Use `key: value` fields, or write a \
                       closure with `|params|` if you meant a block."
            .into(),
        "=" => "Record fields use `key: value` (colon), not `key = value`.".into(),
        other => format!(
            "Expected a record key, found `{other}`. Fields look like `key: value` \
             separated by newlines or commas."
        ),
    }
}

impl<'s> Lower<'_, '_, 's> {
    /// The next token of the text from `*offset` to `end`, lexed with `options`, comments
    /// skipped (nu's `lex_n_tokens_in` for one token), `offset` moved past it; the first lexer
    /// error is kept in `error`, with where that lexing started (where nu's lexer's input
    /// started, which its hint looks back to).
    fn next_record_token(
        &self,
        offset: &mut usize,
        end: usize,
        options: LexOptions,
        error: &mut Option<(LexError, usize)>,
    ) -> Option<Token> {
        loop {
            if *offset >= end {
                return None;
            }
            let rest = WSpan::new(*offset, end);
            let lexed = lex_recovering(
                self.text(rest),
                rest.start,
                options,
                1,
                &mut GroupEnds::new(rest),
            )
            .ok()?;
            if error.is_none()
                && let Some((_, first)) = lexed.errors.first()
            {
                *error = Some((first.clone(), rest.start));
            }
            let token = lexed
                .tokens
                .into_iter()
                .find(|token| token.contents != TokenContents::Eof)?;
            *offset = token.span.end;
            if token.contents != TokenContents::Comment {
                return Some(token);
            }
        }
    }

    /// A record read from the text covering `span` (`parse_record`): its inside lexed a key, a
    /// colon and a value at a time (a spread alone), stopping at a `}` that closes nothing;
    /// each field checked as nu checks it, the first missing colon or value ending the record.
    /// Its type lists the fields when every key is a literal and every spread a record of known
    /// fields.
    pub(super) fn record_text(&mut self, span: WSpan) -> Lowered<Expression> {
        let full = self.span(span);
        let text = self.text(span);
        if !text.starts_with('{') {
            self.working_set.error(ParseError::Expected(
                "{",
                Span::new(full.start, full.start + 1),
            ));
            return Ok(Expression::garbage(self.working_set, full));
        }
        let start = span.start + 1;
        let mut unclosed = !text.ends_with('}');
        let end = if unclosed { span.end } else { span.end - 1 };
        let end = end.max(start);
        let mut extra_tokens = false;
        let mut offset = start;
        let mut lex_error: Option<(LexError, usize)> = None;
        let mut tokens: Vec<Token> = Vec::new();
        while offset < end {
            if let Some((
                LexError::Unbalanced {
                    open: "{",
                    close: "}",
                    ..
                },
                _,
            )) = lex_error
            {
                extra_tokens = true;
                unclosed = false;
                break;
            }
            let Some(key) =
                self.next_record_token(&mut offset, end, LexOptions::RECORD_KEY, &mut lex_error)
            else {
                break;
            };
            tokens.push(key);
            if extract_spread_record(
                self.text(key.span)
                    .as_bytes()
                    .into_spanned(self.span(key.span)),
            )
            .is_some()
            {
                continue;
            }
            let Some(colon) =
                self.next_record_token(&mut offset, end, LexOptions::RECORD_KEY, &mut lex_error)
            else {
                break;
            };
            tokens.push(colon);
            let Some(value) =
                self.next_record_token(&mut offset, end, LexOptions::RECORD_VALUE, &mut lex_error)
            else {
                break;
            };
            tokens.push(value);
        }
        let inner_end = self.span(WSpan::new(end, end)).start;
        if unclosed {
            let open = ParseError::opener_span(full, 1);
            self.working_set.error(ParseError::unclosed(
                "}",
                open,
                Span::new(inner_end, inner_end),
            ));
        } else if extra_tokens {
            let stopped = self.span(WSpan::new(offset, offset)).start;
            self.working_set
                .error(ParseError::ExtraTokensAfterClosingDelimiter(Span::new(
                    stopped, inner_end,
                )));
        }
        if let Some((error, lexed_from)) = &lex_error {
            let error = self.lex_parse_error(error, *lexed_from);
            self.working_set.error(error);
        }

        let mut output = Vec::new();
        let mut field_types = Some(Vec::new());
        let mut names: HashMap<String, Span> = HashMap::new();
        let mut index = 0;
        while index < tokens.len() {
            let current = tokens[index];
            let current_span = self.span(current.span);
            let current_bytes = self.text(current.span).as_bytes();
            if let Some(spread) = extract_spread_record(current_bytes.into_spanned(current_span)) {
                let inner = self.value_text(self.local(spread.span), &SyntaxShape::record())?;
                index += 1;
                match &inner.ty {
                    Type::Record(fields) => {
                        if let Some(field_types) = &mut field_types {
                            field_types.extend(fields.iter().cloned());
                        }
                    }
                    _ => field_types = None,
                }
                let dots = Span::new(current_span.start, current_span.start + 3);
                output.push(RecordItem::Spread(dots, inner));
                continue;
            }
            let field = if current.contents != TokenContents::Item {
                let help = record_key_position_help(self.text(current.span));
                self.working_set.error(ParseError::LabeledErrorWithHelp {
                    error: "Unexpected token in record".into(),
                    label: "expected a record key here".into(),
                    help,
                    span: current_span,
                });
                Expression::garbage(self.working_set, current_span)
            } else {
                let name = w::Expression::new(w::Expr::Garbage, current.span);
                let field = self.field_name(&name)?;
                match check_record_key_or_value(self.working_set, &field, "key") {
                    Some(error) => {
                        self.working_set.error(error);
                        Expression::garbage(self.working_set, field.span)
                    }
                    None => {
                        if let Expr::String(name) | Expr::RawString(name) = &field.expr {
                            match names.entry(name.clone()) {
                                Entry::Occupied(first) => {
                                    self.working_set.error(ParseError::ColumnDefinedTwice(
                                        name.clone(),
                                        field.span,
                                        *first.get(),
                                    ));
                                }
                                Entry::Vacant(entry) => {
                                    entry.insert(field.span);
                                }
                            }
                        }
                        field
                    }
                }
            };
            index += 1;
            let last_end = |this: &Self| this.span(tokens[tokens.len() - 1].span).end;
            if index == tokens.len() {
                let point = Span::new(current_span.end, current_span.end);
                self.working_set.error(ParseError::LabeledErrorWithHelp {
                    error: "Incomplete record field".into(),
                    label: "expected `:` after this key".into(),
                    help: "Record fields look like `key: value`. Add a colon after the key.".into(),
                    span: point,
                });
                let key = Expression::garbage(self.working_set, current_span);
                let value = Expression::garbage(self.working_set, point);
                output.push(RecordItem::Pair(key, value));
                break;
            }
            let colon = tokens[index];
            let colon_span = self.span(colon.span);
            index += 1;
            if self.text(colon.span) != ":" {
                let found = self.text(colon.span);
                self.working_set.error(ParseError::LabeledErrorWithHelp {
                    error: "Expected `:` after record key".into(),
                    label: format!("expected `:`, found `{found}`"),
                    help: "Record fields look like `key: value`. A missing colon often causes this field to be parsed as a block or closure.".into(),
                    span: colon_span,
                });
                let rest = Span::new(colon_span.start, last_end(self));
                let value = Expression::garbage(self.working_set, rest);
                output.push(RecordItem::Pair(field, value));
                break;
            }
            if index == tokens.len() {
                self.working_set.error(ParseError::Expected(
                    "value for record field",
                    Span::new(colon_span.end, colon_span.end),
                ));
                let key = Expression::garbage(
                    self.working_set,
                    Span::new(current_span.start, colon_span.end),
                );
                let rest = Span::new(colon_span.end, last_end(self));
                let value = Expression::garbage(self.working_set, rest);
                output.push(RecordItem::Pair(key, value));
                break;
            }
            let value_token = tokens[index];
            let value_span = self.span(value_token.span);
            let value = if value_token.contents != TokenContents::Item {
                let found = self.text(value_token.span);
                self.working_set.error(ParseError::LabeledErrorWithHelp {
                    error: "Unexpected token in record value".into(),
                    label: format!("expected a value, found `{found}`"),
                    help: "After `key:`, provide a value (string, number, record, list, …).".into(),
                    span: value_span,
                });
                Expression::garbage(self.working_set, value_span)
            } else {
                let value = self.value_text(value_token.span, &SyntaxShape::Any)?;
                match check_record_key_or_value(self.working_set, &value, "value") {
                    Some(error) => {
                        self.working_set.error(error);
                        Expression::garbage(self.working_set, value.span)
                    }
                    None => value,
                }
            };
            index += 1;
            match field.as_string() {
                Some(name) => {
                    if let Some(field_types) = &mut field_types {
                        field_types.push((name, value.ty.clone()));
                    }
                }
                None => field_types = None,
            }
            output.push(RecordItem::Pair(field, value));
        }
        let ty = match field_types {
            Some(fields) => Type::Record(fields.into()),
            None => Type::Any,
        };
        Ok(self.node(Expr::Record(output), full, ty))
    }

    /// A `{...}` item read from its text (`parse_brace_expr`): what its first two tokens make it,
    /// with `shape`: a record, a cell path on a record, a closure, a block or a match block.
    pub(super) fn brace_text(
        &mut self,
        span: WSpan,
        shape: &SyntaxShape,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let full = self.span(span);
        let non_block = |this: &mut Self| {
            this.working_set.error(ParseError::ExpectedWithStringMsg(
                format!("non-block value: {shape}"),
                full,
            ));
            Ok(Expression::garbage(this.working_set, full))
        };
        if span.end <= span.start + 1 {
            return non_block(self);
        }
        // nu's closure, block and match-block readers, as `parse_brace_expr` calls them (a
        // closure without the input type).
        let braces = |this: &mut Self| match shape {
            SyntaxShape::Block => Ok(this.block_text(full, input_type)),
            SyntaxShape::MatchBlock => this.match_block_text(full, input_type),
            _ => this.closure_text(full, shape, None),
        };
        // nu probes the bytes between the first and the last. When the last character takes
        // more than one byte (`{a: 1}╮`, `{é`), nu's probe sees the start of it as item bytes:
        // the inside here keeps the whole character, which lexes alike (item bytes, never a
        // `:`, `}`, pipe or spread), and is never cut inside a character.
        let last = self
            .text(span)
            .chars()
            .next_back()
            .map_or(1, char::len_utf8);
        let inner_end = if last == 1 { span.end - 1 } else { span.end };
        let inner = WSpan::new(span.start + 1, inner_end.max(span.start + 1));
        let probe: Vec<Token> = lex_recovering(
            self.text(inner),
            inner.start,
            LexOptions::BRACE_PROBE,
            2,
            &mut GroupEnds::new(inner),
        )
        .map(|lexed| {
            lexed
                .tokens
                .into_iter()
                .filter(|token| {
                    !matches!(token.contents, TokenContents::Eof | TokenContents::Comment)
                })
                .take(2)
                .collect()
        })
        .unwrap_or_default();
        match probe.as_slice() {
            [] => match shape {
                SyntaxShape::Closure(_) | SyntaxShape::Block | SyntaxShape::MatchBlock => {
                    braces(self)
                }
                _ => self.record_text(span),
            },
            [first, ..]
                if matches!(
                    first.contents,
                    TokenContents::Pipe | TokenContents::PipePipe
                ) =>
            {
                if let SyntaxShape::Block = shape {
                    self.working_set.error(ParseError::Mismatch(
                        "block".into(),
                        "closure".into(),
                        full,
                    ));
                    return Ok(Expression::garbage(self.working_set, full));
                }
                self.closure_text(full, shape, None)
            }
            [_, second, ..] if self.text(second.span) == ":" => {
                self.full_cell_path_text(span, None, None)
            }
            [first, ..] => match shape {
                SyntaxShape::Closure(_) | SyntaxShape::Block | SyntaxShape::MatchBlock => {
                    braces(self)
                }
                // `{}.foo?`.
                _ if self.text(first.span) == "}" => self.full_cell_path_text(span, None, None),
                _ if extract_spread_record(
                    self.text(first.span)
                        .as_bytes()
                        .into_spanned(self.span(first.span)),
                )
                .is_some() =>
                {
                    self.record_text(span)
                }
                SyntaxShape::Any => self.closure_text(full, shape, None),
                _ => non_block(self),
            },
        }
    }
}
