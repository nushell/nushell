use crate::parser::parse2::operator::Operator;
use crate::parser::parse2::span::{Span, Spanned};
use crate::parser::parse2::token_tree::{DelimitedNode, Delimiter, PathNode, TokenNode};
use crate::parser::parse2::tokens::{RawToken, Token};
use crate::parser::parse2::unit::Unit;
use derive_new::new;

#[derive(new)]
pub struct TokenTreeBuilder {
    #[new(default)]
    pos: usize,
}

pub type CurriedToken = Box<dyn FnOnce(&mut TokenTreeBuilder) -> Option<TokenNode> + 'static>;

#[allow(unused)]
impl TokenTreeBuilder {
    pub fn build(block: impl FnOnce(&mut Self) -> TokenNode) -> TokenNode {
        let mut builder = TokenTreeBuilder::new();
        block(&mut builder)
    }

    pub fn op(input: impl Into<Operator>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(input.as_str());

            b.pos = end;

            Some(TokenTreeBuilder::spanned_op(input, (start, end)))
        })
    }

    pub fn spanned_op(input: impl Into<Operator>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(Spanned::from_item(
            RawToken::Operator(input.into()),
            span.into(),
        ))
    }

    pub fn string(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, _) = b.consume("\"");
            let (inner_start, inner_end) = b.consume(&input);
            let (_, end) = b.consume("\"");
            b.pos = end;

            Some(TokenTreeBuilder::spanned_string(
                (inner_start, inner_end),
                (start, end),
            ))
        })
    }

    pub fn spanned_string(input: impl Into<Span>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(Spanned::from_item(
            RawToken::String(input.into()),
            span.into(),
        ))
    }

    pub fn bare(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&input);
            b.pos = end;

            Some(TokenTreeBuilder::spanned_bare((start, end)))
        })
    }

    pub fn spanned_bare(input: impl Into<Span>) -> TokenNode {
        TokenNode::Token(Spanned::from_item(RawToken::Bare, input.into()))
    }

    pub fn int(input: impl Into<i64>) -> CurriedToken {
        let int = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&int.to_string());
            b.pos = end;

            Some(TokenTreeBuilder::spanned_int(int, (start, end)))
        })
    }

    pub fn spanned_int(input: impl Into<i64>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(Token::from_item(RawToken::Integer(input.into()), span))
    }

    pub fn size(int: impl Into<i64>, unit: impl Into<Unit>) -> CurriedToken {
        let int = int.into();
        let unit = unit.into();

        Box::new(move |b| {
            let (start, _) = b.consume(&int.to_string());
            let (_, end) = b.consume(unit.as_str());
            b.pos = end;

            Some(TokenTreeBuilder::spanned_size((int, unit), (start, end)))
        })
    }

    pub fn spanned_size(
        input: (impl Into<i64>, impl Into<Unit>),
        span: impl Into<Span>,
    ) -> TokenNode {
        let (int, unit) = (input.0.into(), input.1.into());

        TokenNode::Token(Spanned::from_item(RawToken::Size(int, unit), span))
    }

    pub fn path(head: CurriedToken, tail: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let start = b.pos;
            let head = head(b).expect("The head of a path must not be whitespace");

            let mut output = vec![];

            for item in tail {
                b.consume(".");

                match item(b) {
                    None => {}
                    Some(v) => output.push(v),
                };
            }

            let end = b.pos;

            Some(TokenTreeBuilder::spanned_path((head, output), (start, end)))
        })
    }

    pub fn spanned_path(input: (TokenNode, Vec<TokenNode>), span: impl Into<Span>) -> TokenNode {
        TokenNode::Path(Spanned::from_item(
            PathNode::new(Box::new(input.0), input.1),
            span,
        ))
    }

    pub fn var(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, _) = b.consume("$");
            let (inner_start, end) = b.consume(&input);

            Some(TokenTreeBuilder::spanned_var(
                (inner_start, end),
                (start, end),
            ))
        })
    }

    pub fn spanned_var(input: impl Into<Span>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(Spanned::from_item(
            RawToken::Variable(input.into()),
            span.into(),
        ))
    }

    pub fn ident(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&input);
            Some(TokenTreeBuilder::spanned_ident((start, end)))
        })
    }

    pub fn spanned_ident(span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(Spanned::from_item(RawToken::Identifier, span.into()))
    }

    pub fn parens(input: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let (start, _) = b.consume("(");
            let mut output = vec![];
            for item in input {
                match item(b) {
                    None => {}
                    Some(v) => output.push(v),
                };
            }

            let (_, end) = b.consume(")");

            Some(TokenNode::Delimited(Spanned::from_item(
                DelimitedNode::new(Delimiter::Paren, output),
                (start, end),
            )))
        })
    }

    pub fn sp() -> CurriedToken {
        Box::new(|b| {
            b.consume(" ");
            None
        })
    }

    pub fn ws(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            b.consume(&input);
            None
        })
    }

    fn consume(&mut self, input: &str) -> (usize, usize) {
        let start = self.pos;
        self.pos += input.len();
        (start, self.pos)
    }
}
