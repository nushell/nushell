use crate::prelude::*;

use crate::parser::parse::flag::{Flag, FlagKind};
use crate::parser::parse::operator::Operator;
use crate::parser::parse::pipeline::{Pipeline, PipelineElement};
use crate::parser::parse::token_tree::{DelimitedNode, Delimiter, PathNode, TokenNode};
use crate::parser::parse::tokens::{RawToken, Token};
use crate::parser::parse::unit::Unit;
use crate::parser::CallNode;
use crate::Span;
use derive_new::new;

#[derive(new)]
pub struct TokenTreeBuilder {}

impl TokenTreeBuilder {
    pub fn spanned_pipeline(
        input: (Vec<PipelineElement>, Option<Span>),
        span: impl Into<Span>,
    ) -> TokenNode {
        TokenNode::Pipeline(Tagged::from_simple_spanned_item(
            Pipeline::new(input.0, input.1.into()),
            span,
        ))
    }

    pub fn spanned_op(input: impl Into<Operator>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Operator(Tagged::from_simple_spanned_item(input.into(), span.into()))
    }

    pub fn spanned_string(input: impl Into<Span>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(Tagged::from_simple_spanned_item(
            RawToken::String(input.into()),
            span.into(),
        ))
    }

    pub fn spanned_bare(input: impl Into<Span>) -> TokenNode {
        TokenNode::Token(Tagged::from_simple_spanned_item(
            RawToken::Bare,
            input.into(),
        ))
    }

    pub fn spanned_external(input: impl Into<Span>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(Tagged::from_simple_spanned_item(
            RawToken::External(input.into()),
            span.into(),
        ))
    }

    pub fn spanned_int(input: impl Into<i64>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(Token::from_simple_spanned_item(
            RawToken::Integer(input.into()),
            span,
        ))
    }

    pub fn spanned_size(
        input: (impl Into<i64>, impl Into<Unit>),
        span: impl Into<Span>,
    ) -> TokenNode {
        let (int, unit) = (input.0.into(), input.1.into());

        TokenNode::Token(Tagged::from_simple_spanned_item(
            RawToken::Size(int, unit),
            span,
        ))
    }

    pub fn spanned_path(input: (TokenNode, Vec<TokenNode>), span: impl Into<Span>) -> TokenNode {
        TokenNode::Path(Tagged::from_simple_spanned_item(
            PathNode::new(Box::new(input.0), input.1),
            span,
        ))
    }

    pub fn spanned_var(input: impl Into<Span>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(Tagged::from_simple_spanned_item(
            RawToken::Variable(input.into()),
            span.into(),
        ))
    }

    pub fn spanned_flag(input: impl Into<Span>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Flag(Tagged::from_simple_spanned_item(
            Flag::new(FlagKind::Longhand, input.into()),
            span.into(),
        ))
    }

    pub fn spanned_shorthand(input: impl Into<Span>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Flag(Tagged::from_simple_spanned_item(
            Flag::new(FlagKind::Shorthand, input.into()),
            span.into(),
        ))
    }

    pub fn spanned_member(span: impl Into<Span>) -> TokenNode {
        TokenNode::Member(span.into())
    }

    pub fn spanned_call(input: Vec<TokenNode>, span: impl Into<Span>) -> Tagged<CallNode> {
        if input.len() == 0 {
            panic!("BUG: spanned call (TODO)")
        }

        let mut input = input.into_iter();

        let head = input.next().unwrap();
        let tail = input.collect();

        Tagged::from_simple_spanned_item(CallNode::new(Box::new(head), tail), span)
    }

    pub fn spanned_parens(input: impl Into<Vec<TokenNode>>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Delimited(Tagged::from_simple_spanned_item(
            DelimitedNode::new(Delimiter::Paren, input.into()),
            span,
        ))
    }

    pub fn spanned_square(input: impl Into<Vec<TokenNode>>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Delimited(Tagged::from_simple_spanned_item(
            DelimitedNode::new(Delimiter::Square, input.into()),
            span,
        ))
    }

    pub fn spanned_brace(input: impl Into<Vec<TokenNode>>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Delimited(Tagged::from_simple_spanned_item(
            DelimitedNode::new(Delimiter::Brace, input.into()),
            span,
        ))
    }

    pub fn spanned_ws(span: impl Into<Span>) -> TokenNode {
        let span = span.into();

        TokenNode::Whitespace(span.into())
    }
}
