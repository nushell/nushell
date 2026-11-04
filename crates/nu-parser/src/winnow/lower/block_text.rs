//! Closures, blocks and match blocks read from the text of their `{...}` item, as nu-parser's
//! `parse_closure_expression`, `parse_block_expression` and `parse_match_block_expression`
//! read them, where the winnow parser read the braces otherwise (a record, a cell path) or went
//! on past an error in them: the braces checked, the inside lexed, a closure's parameters read
//! from between its `|`s and the statements after them read by the winnow parser as nu's
//! `parse_block` hands them to it ([`super::lexed_block`]); a match block's arms read item by
//! item.

use std::sync::Arc;

use nu_protocol::{
    ParseError, Signature, Span, SyntaxShape, Type, TypeSet,
    ast::{Block, Expr, Expression, MatchPattern, Pattern},
};

use super::{BlockOf, Lower, Lowered, lexed_block};
use crate::parse_captures_compile::compile_block;
use crate::winnow::syntax::Span as WSpan;
use crate::winnow::syntax::lex::{LexOptions, Token, TokenContents};

/// A closure's parameters as its text gives them: the signature and the span it is read from
/// (`|...|`, or `||`).
type ClosureParams = Option<(Box<Signature>, Span)>;

impl Lower<'_, '_, '_> {
    /// The tokens inside the braces of the item at `span`, lexed as a block's (the first error
    /// reported), the span inside and whether the closing brace is there (an unclosed one
    /// reported); `None` when the item does not start with `{`, reported as no `expected`.
    fn brace_block_tokens(
        &mut self,
        span: Span,
        expected: &'static str,
    ) -> Option<(Vec<Token>, Span, bool)> {
        let text = self.text_at(span);
        if !text.starts_with('{') {
            self.working_set.error(ParseError::Expected(expected, span));
            return None;
        }
        let closed = text.ends_with('}');
        let end = if closed {
            span.end - 1
        } else {
            let open = ParseError::opener_span(span, 1);
            self.working_set
                .error(ParseError::unclosed("}", open, Span::point(span.end)));
            span.end
        };
        let inner = Span::new(span.start + 1, end);
        let (tokens, error) = self.lex_span(inner, LexOptions::BLOCK);
        if let Some(error) = error {
            self.working_set.error(error);
        }
        Some((tokens, inner, closed))
    }

    /// `tokens`, lexed by [`Self::lex_span`] with an end of line for each line break between them:
    /// that follows nu's lexer, which folds the end of a line before a `|` into it, and the winnow
    /// parser reads a line-leading `|` from the end of line before it.
    fn with_line_ends(&self, tokens: &[Token]) -> Vec<Token> {
        let mut lines = Vec::with_capacity(tokens.len());
        let mut previous_end = None;
        for token in tokens {
            if let Some(end) = previous_end {
                let gap = &self.source[end..token.span.start];
                lines.extend(gap.match_indices('\n').map(|(at, _)| Token {
                    contents: TokenContents::Eol,
                    span: WSpan::new(end + at, end + at + 1),
                }));
            }
            lines.push(*token);
            previous_end = Some(token.span.end);
        }
        lines
    }

    /// The block of the statements `tokens`, inside the item at `span`, read as nu's
    /// `parse_block` reads tokens it is handed, not scoped of its own: of a closure's or a
    /// block's braces (`BlockOf::Code`), or of an assignment's right side (a subexpression).
    pub(super) fn items_block(
        &mut self,
        tokens: &[Token],
        span: Span,
        of: BlockOf,
        input_type: Option<&Type>,
    ) -> Block {
        // The item's text, as nu's `parse_block` hands it to the winnow parser.
        let item = self.local(span);
        let unfolded;
        let tokens = match of {
            BlockOf::Code => {
                unfolded = self.with_line_ends(tokens);
                unfolded.as_slice()
            }
            _ => tokens,
        };
        let tokens = tokens
            .iter()
            .map(|token| Token {
                span: WSpan::new(token.span.start - item.start, token.span.end - item.start),
                ..*token
            })
            .collect();
        let source = self.text(item);
        lexed_block(
            self.working_set,
            source,
            span.start,
            tokens,
            span,
            false,
            of,
            input_type,
        )
    }

    /// A closure read from the `{...}` item at `span` (`parse_closure_expression`), in a scope
    /// of its own: its parameters, checked against those a `closure(...)` shape expects, its
    /// body receiving `input_type`, compiled unless the working set holds a parse error or it
    /// is the body of the `def` being lowered. An item without its closing brace leaves its
    /// scope open, as there.
    pub(super) fn closure_text(
        &mut self,
        span: Span,
        shape: &SyntaxShape,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let Some((tokens, inner, closed)) = self.brace_block_tokens(span, "closure") else {
            return Ok(Expression::garbage(self.working_set, span));
        };
        self.working_set.enter_scope();
        let (signature, skip) = match self.closure_params(&tokens, inner) {
            Ok(params) => params,
            Err(unlowered) => {
                self.working_set.exit_scope();
                return Err(unlowered);
            }
        };
        if let Some((signature, params_span)) = &signature {
            self.check_closure_params(shape, signature, *params_span);
        }
        let mut block = self.items_block(&tokens[skip..], span, BlockOf::Code, input_type);
        if self.working_set.parse_errors.is_empty() && self.working_set.def_body_span != Some(span)
        {
            compile_block(self.working_set, &mut block);
        }
        if let Some((signature, _)) = signature {
            block.signature = signature;
        }
        block.span = Some(span);
        block.scope_bindings = self.working_set.snapshot_scope_bindings();
        if closed {
            self.working_set.exit_scope();
        }
        let block_id = self.working_set.add_block(Arc::new(block));
        Ok(self.node(Expr::Closure(block_id), span, Type::Closure))
    }

    /// A closure's parameters at the start of its `tokens`, inside its braces at `inner`:
    /// between a `|` and the next one (with no next one, to the end, reported), or none after
    /// `||`; with how many tokens they take.
    fn closure_params(&mut self, tokens: &[Token], inner: Span) -> Lowered<(ClosureParams, usize)> {
        let Some(first) = tokens.first() else {
            return Ok((None, 0));
        };
        let open = self.span(first.span);
        match first.contents {
            TokenContents::Pipe => {
                let close = tokens[1..]
                    .iter()
                    .position(|token| token.contents == TokenContents::Pipe);
                let (end, skip) = match close {
                    Some(index) => (self.span(tokens[index + 1].span).end, index + 2),
                    None => {
                        let opener = Span::new(open.start, (open.start + 1).min(inner.end));
                        let end = Span::point(inner.end);
                        self.working_set
                            .error(ParseError::unclosed("|", opener, end));
                        (inner.end, 1)
                    }
                };
                let params_span = Span::new(open.start, end);
                let signature = self.signature_items(params_span, false)?;
                Ok((Some((signature, params_span)), skip))
            }
            TokenContents::PipePipe => Ok((Some((Box::new(Signature::new("closure")), open)), 1)),
            _ => Ok((None, 0)),
        }
    }

    /// A match block read from the `{...}` item at `span` (`parse_match_block_expression`):
    /// arms `pattern => result`, an or-pattern's alternatives between `|`s and a guard after
    /// `if`, each arm in a scope of its own, its result read as a block or an expression
    /// receiving `input_type`. Its type is the union of the results', with `nothing` when no arm
    /// is a wildcard. Each arm missing a part is reported; a guard with no expression makes the
    /// whole block garbage. As there, an item without its closing brace leaves its arms' scopes
    /// open, and a guard with no expression its arm's.
    pub(super) fn match_block_text(
        &mut self,
        span: Span,
        input_type: Option<&Type>,
    ) -> Lowered<Expression> {
        let text = self.text_at(span);
        if !text.starts_with('{') {
            self.working_set
                .error(ParseError::Expected("closure", span));
            return Ok(Expression::garbage(self.working_set, span));
        }
        let closed = text.ends_with('}');
        let end = if closed {
            span.end - 1
        } else {
            let open = ParseError::opener_span(span, 1);
            self.working_set
                .error(ParseError::unclosed("}", open, Span::point(span.end)));
            span.end
        };
        let (tokens, error) = self.lex_span(Span::new(span.start + 1, end), LexOptions::MATCH);
        if let Some(error) = error {
            self.working_set.error(error);
        }
        let items: Vec<Span> = tokens
            .iter()
            .filter(|token| token.contents != TokenContents::Comment)
            .map(|token| self.span(token.span))
            .collect();
        // A scope a failed reading leaves open goes; nu's never fails.
        let scopes = self.working_set.delta.scope.len();
        let arms = self.match_arms_text(&items, span, closed, input_type);
        let arms = match arms {
            Ok(Some(arms)) => arms,
            Ok(None) => return Ok(Expression::garbage(self.working_set, span)),
            Err(unlowered) => {
                self.take_open_scopes(scopes);
                return Err(unlowered);
            }
        };
        let mut output = Type::one_of([]);
        for (_, result) in &arms {
            output = output.union(self.branch_type(result, input_type));
        }
        if !arms.iter().any(|(pattern, _)| pattern.is_wildcard()) {
            output = output.union(Type::Nothing);
        }
        Ok(self.node(Expr::MatchBlock(arms), span, output))
    }

    /// The arms of the match block at `span` from its `items` ([`Self::match_block_text`]);
    /// `None` for a guard with no expression.
    fn match_arms_text(
        &mut self,
        items: &[Span],
        span: Span,
        closed: bool,
        input_type: Option<&Type>,
    ) -> Lowered<Option<Vec<(MatchPattern, Expression)>>> {
        let end_of = |index: usize| Span::point(items[index].end);
        let mut arms = Vec::new();
        let mut position = 0;
        while position < items.len() {
            self.working_set.enter_scope();
            let mut pattern = self.pattern_text(items[position])?;
            position += 1;
            if position >= items.len() {
                let at = end_of(position - 1);
                self.working_set.error(ParseError::Mismatch(
                    "=>".into(),
                    "end of input".into(),
                    at,
                ));
                self.working_set.exit_scope();
                break;
            }
            let mut connector = self.text_at(items[position]);
            // Alternatives between `|`s.
            if connector == "|" {
                let mut alternatives = vec![pattern];
                while connector == "|" && position < items.len() {
                    connector = "";
                    position += 1;
                    if position >= items.len() {
                        let at = end_of(position - 1);
                        self.working_set.error(ParseError::Mismatch(
                            "pattern".into(),
                            "end of input".into(),
                            at,
                        ));
                        break;
                    }
                    alternatives.push(self.pattern_text(items[position])?);
                    position += 1;
                    if position >= items.len() {
                        let at = end_of(position - 1);
                        self.working_set.error(ParseError::Mismatch(
                            "=>".into(),
                            "end of input".into(),
                            at,
                        ));
                        break;
                    }
                    connector = self.text_at(items[position]);
                }
                let start = alternatives
                    .first()
                    .map_or(span.start, |first| first.span.start);
                let end = alternatives.last().map_or(span.start, |last| last.span.end);
                pattern = MatchPattern {
                    pattern: Pattern::Or(alternatives),
                    guard: None,
                    span: Span::new(start, end),
                };
            }
            // A guard, the items up to `=>` read as a math expression.
            if connector == "if" {
                let if_end = Span::point(items[position].end);
                position += 1;
                let guard_error = ParseError::LabeledErrorWithHelp {
                    error: "Match guard without an expression".into(),
                    label: "expected an expression".into(),
                    help: "The `if` keyword must be followed with an expression".into(),
                    span: if_end,
                };
                let arrow = items[position.min(items.len())..]
                    .iter()
                    .position(|item| self.text_at(*item) == "=>");
                if position >= items.len() || arrow == Some(0) {
                    self.working_set.error(guard_error);
                    return Ok(None);
                }
                let guard_items = match arrow {
                    Some(arrow) => &items[position..position + arrow],
                    None => &items[position..],
                };
                // `parse_multispan_value` reads it with `parse_math_expression`, all of it.
                let guard_items: Vec<_> =
                    guard_items.iter().map(|item| self.local(*item)).collect();
                let guard = self.math_items(&guard_items, None, None)?;
                let after = guard_items.len();
                pattern.guard = Some(Box::new(guard));
                // nu's index of the last item the guard read, past the `=>` when there is one.
                let last = after.saturating_sub(1);
                position += if arrow.is_some() { last + 1 } else { last };
                connector = items.get(position).map_or("", |item| self.text_at(*item));
            }
            if connector == "=>" {
                position += 1;
            } else {
                let at = end_of(position - 1);
                self.working_set.error(ParseError::Mismatch(
                    "=>".into(),
                    "end of input".into(),
                    at,
                ));
            }
            if position >= items.len() {
                let at = end_of(position - 1);
                self.working_set.error(ParseError::Mismatch(
                    "match result".into(),
                    "end of input".into(),
                    at,
                ));
                self.working_set.exit_scope();
                break;
            }
            let shape = SyntaxShape::OneOf(vec![SyntaxShape::Block, SyntaxShape::Expression]);
            let (result, _) =
                self.multispan_value_text(&items[position..=position], &shape, input_type)?;
            position += 1;
            if closed {
                self.working_set.exit_scope();
            }
            arms.push((pattern, result));
        }
        Ok(Some(arms))
    }

    /// A block read from the `{...}` item at `span` (`parse_block_expression`), in a scope of
    /// its own, its statements receiving `input_type`; a `|` first is reported (a closure's
    /// parameters). Not compiled on its own but with the block it is in. An item without its
    /// closing brace leaves its scope open, as there.
    pub(super) fn block_text(&mut self, span: Span, input_type: Option<&Type>) -> Expression {
        let Some((tokens, _, closed)) = self.brace_block_tokens(span, "block") else {
            return Expression::garbage(self.working_set, span);
        };
        self.working_set.enter_scope();
        if let Some(first) = tokens.first()
            && first.contents == TokenContents::Pipe
        {
            let at = self.span(first.span);
            self.working_set
                .error(ParseError::Expected("block but found closure", at));
        }
        let mut block = self.items_block(&tokens, span, BlockOf::Code, input_type);
        block.span = Some(span);
        block.scope_bindings = self.working_set.snapshot_scope_bindings();
        if closed {
            self.working_set.exit_scope();
        }
        let block_id = self.working_set.add_block(Arc::new(block));
        self.node(Expr::Block(block_id), span, Type::Block)
    }
}
