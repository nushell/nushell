use crate::hir::syntax_shape::flat_shape::ShapeResult;
use crate::hir::syntax_shape::ExpandContext;
use crate::hir::tokens_iterator::TokensIterator;
use crate::parse::token_tree::SpannedToken;

use getset::Getters;
use nu_errors::ParseError;
use nu_protocol::SpannedTypeName;
use nu_source::Span;
use std::sync::Arc;

#[derive(Getters, Debug, Clone)]
pub struct TokensIteratorState<'content> {
    pub(crate) tokens: &'content [SpannedToken],
    pub(crate) span: Span,
    pub(crate) index: usize,
    pub(crate) seen: indexmap::IndexSet<usize>,
    #[get = "pub"]
    pub(crate) shapes: Vec<ShapeResult>,
    pub(crate) errors: indexmap::IndexMap<Span, Vec<String>>,
    pub(crate) context: Arc<ExpandContext<'content>>,
}

#[derive(Debug)]
pub struct Peeked<'content, 'me> {
    pub(crate) node: Option<&'content SpannedToken>,
    pub(crate) iterator: &'me mut TokensIterator<'content>,
    pub(crate) from: usize,
    pub(crate) to: usize,
}

impl<'content, 'me> Peeked<'content, 'me> {
    pub fn commit(&mut self) -> Option<&'content SpannedToken> {
        let Peeked {
            node,
            iterator,
            from,
            to,
        } = self;

        let node = (*node)?;
        iterator.commit(*from, *to);
        Some(node)
    }

    pub fn rollback(self) {}

    pub fn not_eof(self, expected: &str) -> Result<PeekedNode<'content, 'me>, ParseError> {
        match self.node {
            None => Err(ParseError::unexpected_eof(
                expected.to_string(),
                self.iterator.eof_span(),
            )),
            Some(node) => Ok(PeekedNode {
                node,
                iterator: self.iterator,
                from: self.from,
                to: self.to,
            }),
        }
    }

    pub fn type_error(&self, expected: &'static str) -> ParseError {
        peek_error(&self.node, self.iterator.eof_span(), expected)
    }
}

#[derive(Debug)]
pub struct PeekedNode<'content, 'me> {
    pub(crate) node: &'content SpannedToken,
    pub(crate) iterator: &'me mut TokensIterator<'content>,
    from: usize,
    to: usize,
}

impl<'content, 'me> PeekedNode<'content, 'me> {
    pub fn commit(self) -> &'content SpannedToken {
        let PeekedNode {
            node,
            iterator,
            from,
            to,
        } = self;

        iterator.commit(from, to);
        node
    }

    pub fn rollback(self) {}

    pub fn type_error(&self, expected: &'static str) -> ParseError {
        peek_error(&Some(self.node), self.iterator.eof_span(), expected)
    }
}

pub fn peek_error(
    node: &Option<&SpannedToken>,
    eof_span: Span,
    expected: &'static str,
) -> ParseError {
    match node {
        None => ParseError::unexpected_eof(expected, eof_span),
        Some(node) => ParseError::mismatch(expected, node.spanned_type_name()),
    }
}
