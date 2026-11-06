//! Match patterns read from their text, as nu-parser's `parse_patterns.rs` reads them: a
//! variable (`$x`), a record (`{a: $x, $b}`), a list (`[1 ..$rest]`), `_`, or a value read with
//! any shape and evaluated as a constant. A pattern's variables are declared in the arm's scope.

use nu_protocol::{
    ParseError, Span, SyntaxShape, Type, VarId,
    ast::{Expr, MatchPattern, Pattern},
    eval_const::eval_constant,
};

use super::{Lower, Lowered};
use crate::parse_signatures::ensure_not_reserved_variable_name;
use crate::winnow::syntax::lex::{LexOptions, Token, TokenContents};
use crate::winnow::syntax::parse_helpers::is_variable;

/// A pattern that matches nothing, over `span`, which reported why.
fn garbage(span: Span) -> MatchPattern {
    MatchPattern {
        pattern: Pattern::Garbage,
        guard: None,
        span,
    }
}

/// `pattern` over `span`, without a guard.
fn pattern_at(pattern: Pattern, span: Span) -> MatchPattern {
    MatchPattern {
        pattern,
        guard: None,
        span,
    }
}

impl Lower<'_, '_, '_> {
    /// The pattern of the item at `span` (`parse_pattern`), what its first byte makes it.
    pub(super) fn pattern_text(&mut self, span: Span) -> Lowered<MatchPattern> {
        let text = self.text_at(span);
        if text.starts_with('$') {
            Ok(self.variable_pattern(span))
        } else if text.starts_with('{') {
            self.record_pattern(span)
        } else if text.starts_with('[') {
            self.list_pattern(span)
        } else if text == "_" {
            Ok(pattern_at(Pattern::IgnoreValue, span))
        } else {
            self.value_pattern(span)
        }
    }

    /// A literal or expression pattern (`parse_value_pattern`): the item read with any shape
    /// and evaluated as a constant, which it must be (reported over garbage); an item that
    /// reads as garbage reported its error already.
    fn value_pattern(&mut self, span: Span) -> Lowered<MatchPattern> {
        let value = self.value_text(self.local(span), &SyntaxShape::Any)?;
        if matches!(value.expr, Expr::Garbage) {
            return Ok(garbage(span));
        }
        Ok(match eval_constant(self.working_set, &value) {
            Ok(value) => pattern_at(Pattern::Value(value), span),
            Err(error) => {
                let error = error.wrap(self.working_set, span);
                self.working_set.error(error);
                garbage(span)
            }
        })
    }

    /// The variable the item at `span` names, the current frame's of that name or a new one
    /// (`parse_variable_pattern_helper`); `None` for no variable name.
    fn pattern_variable(&mut self, span: Span) -> Option<VarId> {
        let name = self.text_at(span);
        if !is_variable(name) {
            return None;
        }
        if let Some(var_id) = self
            .working_set
            .find_variable_in_current_frame(name.as_bytes())
        {
            return Some(var_id);
        }
        ensure_not_reserved_variable_name(self.working_set, name.as_bytes(), span);
        let name = name.as_bytes().to_vec();
        Some(self.working_set.add_variable(name, span, Type::Any, false))
    }

    /// A variable pattern (`parse_variable_pattern`); no variable name is reported.
    fn variable_pattern(&mut self, span: Span) -> MatchPattern {
        match self.pattern_variable(span) {
            Some(var_id) => pattern_at(Pattern::Variable(var_id), span),
            None => {
                self.working_set
                    .error(ParseError::Expected("valid variable name", span));
                garbage(span)
            }
        }
    }

    /// The tokens inside the brackets of the item at `span` (its opener checked by the
    /// caller), lexed with `options` (the first error reported), comments dropped; a missing
    /// closer `close` reported.
    fn pattern_tokens(&mut self, span: Span, close: char, options: LexOptions) -> Vec<Token> {
        let text = self.text_at(span);
        let start = span.start + usize::from(text.starts_with(['[', '{']));
        let end = if text.ends_with(close) {
            span.end - 1
        } else {
            let open = ParseError::opener_span(span, 1);
            let closer = if close == ']' { "]" } else { "}" };
            self.working_set
                .error(ParseError::unclosed(closer, open, Span::point(span.end)));
            span.end
        };
        let (tokens, error) = self.lex_span(Span::new(start, end.max(start)), options);
        if let Some(error) = error {
            self.working_set.error(error);
        }
        tokens
            .into_iter()
            .filter(|token| token.contents != TokenContents::Comment)
            .collect()
    }

    /// A list pattern (`parse_list_pattern`): its items lite-parsed (a `;` reported), each a
    /// pattern, `..` (the rest ignored) or `..$name` (the rest bound), one rest at most.
    fn list_pattern(&mut self, span: Span) -> Lowered<MatchPattern> {
        let tokens = self.pattern_tokens(span, ']', LexOptions::PATTERN_LIST);
        if let Some(semicolon) = tokens
            .iter()
            .find(|token| token.contents == TokenContents::Semicolon)
        {
            let at = self.span(semicolon.span);
            self.working_set.error(ParseError::LabeledErrorWithHelp {
                error: "Unexpected semicolon in list pattern".into(),
                label: "not a valid list separator".into(),
                help: "Use commas or whitespace to separate list items.".into(),
                span: at,
            });
        }
        let (block, error) = self.lite_parse(&tokens);
        if let Some(error) = error {
            self.working_set.error(error);
        }
        let mut items = Vec::new();
        let mut has_rest = false;
        let parts = block
            .pipelines()
            .next()
            .into_iter()
            .flatten()
            .flat_map(|command| block.parts(command));
        for part in parts {
            let part = self.span(part.span);
            let text = self.text_at(part);
            let item = if text == ".." {
                pattern_at(Pattern::IgnoreRest, part)
            } else if text.starts_with("..$") {
                match self.pattern_variable(Span::new(part.start + 2, part.end)) {
                    Some(var_id) => pattern_at(Pattern::Rest(var_id), part),
                    None => {
                        self.working_set
                            .error(ParseError::Expected("valid variable name", part));
                        garbage(part)
                    }
                }
            } else {
                self.pattern_text(part)?
            };
            let is_rest = matches!(item.pattern, Pattern::IgnoreRest | Pattern::Rest(_));
            if is_rest && has_rest {
                // The extra `..$name`'s variable is declared already, so the arm's body
                // reports only this.
                self.working_set.error(ParseError::LabeledErrorWithHelp {
                    error: "`..` can only be used once per list pattern".into(),
                    label: "extra rest pattern".into(),
                    help: "Keep one `..` or `..$name` and remove the others.".into(),
                    span: part,
                });
                items.push(garbage(part));
            } else {
                has_rest |= is_rest;
                items.push(item);
            }
        }
        Ok(pattern_at(Pattern::List(items), span))
    }

    /// A record pattern (`parse_record_pattern`): fields `name: pattern`, or `$name`, a field
    /// and a variable of that name; a field without its `:` and pattern is reported, the whole
    /// pattern garbage.
    fn record_pattern(&mut self, span: Span) -> Lowered<MatchPattern> {
        if !self.text_at(span).starts_with('{') {
            let at = Span::new(span.start, span.start + 1);
            self.working_set.error(ParseError::Expected("{", at));
        }
        let tokens = self.pattern_tokens(span, '}', LexOptions::PATTERN_RECORD);
        let mut fields = Vec::new();
        let mut index = 0;
        while index < tokens.len() {
            let field_span = self.span(tokens[index].span);
            let text = self.text_at(field_span);
            let field = if let Some(name) = text.strip_prefix('$') {
                (name.to_string(), self.variable_pattern(field_span))
            } else {
                let colon = tokens.get(index + 1).map(|token| self.text(token.span));
                let Some(value) = tokens.get(index + 2).filter(|_| colon == Some(":")) else {
                    self.working_set.error(ParseError::Expected("record", span));
                    return Ok(garbage(span));
                };
                index += 2;
                let pattern = self.pattern_text(self.span(value.span))?;
                (text.to_string(), pattern)
            };
            fields.push(field);
            index += 1;
        }
        Ok(pattern_at(Pattern::Record(fields), span))
    }
}
