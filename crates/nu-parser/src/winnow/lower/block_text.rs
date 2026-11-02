//! Closures and blocks read from the text of their `{...}` item, as nu-parser's
//! `parse_closure_expression` and `parse_block_expression` read them, where the winnow parser
//! read the braces otherwise (a record, a cell path) or went on past an error in them: the
//! braces checked, the inside lexed as a block's, a closure's parameters read from between its
//! `|`s, and the statements after them read by the winnow parser as nu's `parse_block` hands
//! them to it ([`super::lexed_block`]).

use std::sync::Arc;

use nu_protocol::{
    ParseError, Signature, Span, SyntaxShape, Type,
    ast::{Block, Expr, Expression},
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

    /// The block of the statements `tokens`, inside the braces of the item at `span`, read as
    /// `parse_closure_expression` and `parse_block_expression` read it: not scoped of its own,
    /// no subexpression.
    fn brace_body(&mut self, tokens: &[Token], span: Span, input_type: Option<&Type>) -> Block {
        // The item's text, as nu's `parse_block` hands it to the winnow parser.
        let item = self.local(span);
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
            BlockOf::Code,
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
        let mut block = self.brace_body(&tokens[skip..], span, input_type);
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
        let mut block = self.brace_body(&tokens, span, input_type);
        block.span = Some(span);
        block.scope_bindings = self.working_set.snapshot_scope_bindings();
        if closed {
            self.working_set.exit_scope();
        }
        let block_id = self.working_set.add_block(Arc::new(block));
        self.node(Expr::Block(block_id), span, Type::Block)
    }
}
