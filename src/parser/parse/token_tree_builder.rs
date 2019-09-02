#[allow(unused)]
use crate::prelude::*;

use crate::parser::parse::flag::{Flag, FlagKind};
use crate::parser::parse::operator::Operator;
use crate::parser::parse::pipeline::{Pipeline, PipelineElement};
use crate::parser::parse::token_tree::{DelimitedNode, Delimiter, PathNode, TokenNode};
use crate::parser::parse::tokens::{RawNumber, RawToken};
use crate::parser::parse::unit::Unit;
use crate::parser::CallNode;
use crate::Span;
use derive_new::new;

#[derive(new)]
pub struct TokenTreeBuilder {
    #[new(default)]
    pos: usize,
}

#[allow(unused)]
pub type CurriedNode<T> = Box<dyn FnOnce(&mut TokenTreeBuilder) -> T + 'static>;
pub type CurriedToken = Box<dyn FnOnce(&mut TokenTreeBuilder) -> TokenNode + 'static>;
pub type CurriedCall = Box<dyn FnOnce(&mut TokenTreeBuilder) -> Tagged<CallNode> + 'static>;

#[allow(unused)]
impl TokenTreeBuilder {
    pub fn build(block: impl FnOnce(&mut Self) -> TokenNode) -> TokenNode {
        let mut builder = TokenTreeBuilder::new();
        block(&mut builder)
    }

    pub fn pipeline(input: Vec<(Option<&str>, CurriedCall, Option<&str>)>) -> CurriedToken {
        let input: Vec<(Option<String>, CurriedCall, Option<String>)> = input
            .into_iter()
            .map(|(pre, call, post)| {
                (
                    pre.map(|s| s.to_string()),
                    call,
                    post.map(|s| s.to_string()),
                )
            })
            .collect();

        Box::new(move |b| {
            let start = b.pos;

            let mut out: Vec<PipelineElement> = vec![];

            let mut input = input.into_iter().peekable();
            let (pre, call, post) = input
                .next()
                .expect("A pipeline must contain at least one element");

            let pre_span = pre.map(|pre| b.consume(&pre));
            let call = call(b);
            let post_span = post.map(|post| b.consume(&post));
            let pipe = input.peek().map(|_| Span::from(b.consume("|")));
            out.push(PipelineElement::new(
                pre_span.map(Span::from),
                call,
                post_span.map(Span::from),
                pipe,
            ));

            loop {
                match input.next() {
                    None => break,
                    Some((pre, call, post)) => {
                        let pre_span = pre.map(|pre| b.consume(&pre));
                        let call = call(b);
                        let post_span = post.map(|post| b.consume(&post));

                        let pipe = input.peek().map(|_| Span::from(b.consume("|")));

                        out.push(PipelineElement::new(
                            pre_span.map(Span::from),
                            call,
                            post_span.map(Span::from),
                            pipe,
                        ));
                    }
                }
            }

            let end = b.pos;

            TokenTreeBuilder::spanned_pipeline((out, None), (start, end))
        })
    }

    pub fn spanned_pipeline(
        input: (Vec<PipelineElement>, Option<Span>),
        span: impl Into<Span>,
    ) -> TokenNode {
        TokenNode::Pipeline(Tagged::from_simple_spanned_item(
            Pipeline::new(input.0, input.1.into()),
            span,
        ))
    }

    pub fn op(input: impl Into<Operator>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(input.as_str());

            b.pos = end;

            TokenTreeBuilder::spanned_op(input, (start, end))
        })
    }

    pub fn spanned_op(input: impl Into<Operator>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Operator(Tagged::from_simple_spanned_item(input.into(), span.into()))
    }

    pub fn string(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, _) = b.consume("\"");
            let (inner_start, inner_end) = b.consume(&input);
            let (_, end) = b.consume("\"");
            b.pos = end;

            TokenTreeBuilder::spanned_string((inner_start, inner_end), (start, end))
        })
    }

    pub fn spanned_string(input: impl Into<Span>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(Tagged::from_simple_spanned_item(
            RawToken::String(input.into()),
            span.into(),
        ))
    }

    pub fn bare(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&input);
            b.pos = end;

            TokenTreeBuilder::spanned_bare((start, end))
        })
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

    pub fn int(input: impl Into<BigInt>) -> CurriedToken {
        let int = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&int.to_string());
            b.pos = end;

            TokenTreeBuilder::spanned_number(RawNumber::Int((start, end).into()), (start, end))
        })
    }

    pub fn decimal(input: impl Into<BigDecimal>) -> CurriedToken {
        let decimal = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&decimal.to_string());
            b.pos = end;

            TokenTreeBuilder::spanned_number(RawNumber::Decimal((start, end).into()), (start, end))
        })
    }

    pub fn spanned_number(input: impl Into<RawNumber>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(Tagged::from_simple_spanned_item(
            RawToken::Number(input.into()),
            span.into(),
        ))
    }

    pub fn size(int: impl Into<i64>, unit: impl Into<Unit>) -> CurriedToken {
        let int = int.into();
        let unit = unit.into();

        Box::new(move |b| {
            let (start_int, end_int) = b.consume(&int.to_string());
            let (start_unit, end_unit) = b.consume(unit.as_str());
            b.pos = end_unit;

            TokenTreeBuilder::spanned_size(
                (RawNumber::Int((start_int, end_int).into()), unit),
                (start_int, end_unit),
            )
        })
    }

    pub fn spanned_size(
        input: (impl Into<RawNumber>, impl Into<Unit>),
        span: impl Into<Span>,
    ) -> TokenNode {
        let (int, unit) = (input.0.into(), input.1.into());

        TokenNode::Token(Tagged::from_simple_spanned_item(
            RawToken::Size(int, unit),
            span,
        ))
    }

    pub fn path(head: CurriedToken, tail: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let start = b.pos;
            let head = head(b);

            let mut output = vec![];

            for item in tail {
                b.consume(".");

                output.push(item(b));
            }

            let end = b.pos;

            TokenTreeBuilder::spanned_path((head, output), (start, end))
        })
    }

    pub fn spanned_path(input: (TokenNode, Vec<TokenNode>), span: impl Into<Span>) -> TokenNode {
        TokenNode::Path(Tagged::from_simple_spanned_item(
            PathNode::new(Box::new(input.0), input.1),
            span,
        ))
    }

    pub fn var(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, _) = b.consume("$");
            let (inner_start, end) = b.consume(&input);

            TokenTreeBuilder::spanned_var((inner_start, end), (start, end))
        })
    }

    pub fn spanned_var(input: impl Into<Span>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(Tagged::from_simple_spanned_item(
            RawToken::Variable(input.into()),
            span.into(),
        ))
    }

    pub fn flag(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, _) = b.consume("--");
            let (inner_start, end) = b.consume(&input);

            TokenTreeBuilder::spanned_flag((inner_start, end), (start, end))
        })
    }

    pub fn spanned_flag(input: impl Into<Span>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Flag(Tagged::from_simple_spanned_item(
            Flag::new(FlagKind::Longhand, input.into()),
            span.into(),
        ))
    }

    pub fn shorthand(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, _) = b.consume("-");
            let (inner_start, end) = b.consume(&input);

            TokenTreeBuilder::spanned_shorthand((inner_start, end), (start, end))
        })
    }

    pub fn spanned_shorthand(input: impl Into<Span>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Flag(Tagged::from_simple_spanned_item(
            Flag::new(FlagKind::Shorthand, input.into()),
            span.into(),
        ))
    }

    pub fn member(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&input);
            TokenTreeBuilder::spanned_member((start, end))
        })
    }

    pub fn spanned_member(span: impl Into<Span>) -> TokenNode {
        TokenNode::Member(span.into())
    }

    pub fn call(head: CurriedToken, input: Vec<CurriedToken>) -> CurriedCall {
        Box::new(move |b| {
            let start = b.pos;

            let head_node = head(b);

            let mut nodes = vec![head_node];
            for item in input {
                nodes.push(item(b));
            }

            let end = b.pos;

            TokenTreeBuilder::spanned_call(nodes, (start, end))
        })
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

    pub fn parens(input: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let (start, _) = b.consume("(");
            let mut output = vec![];
            for item in input {
                output.push(item(b));
            }

            let (_, end) = b.consume(")");

            TokenTreeBuilder::spanned_parens(output, (start, end))
        })
    }

    pub fn spanned_parens(input: impl Into<Vec<TokenNode>>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Delimited(Tagged::from_simple_spanned_item(
            DelimitedNode::new(Delimiter::Paren, input.into()),
            span,
        ))
    }

    pub fn square(input: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let (start, _) = b.consume("[");
            let mut output = vec![];
            for item in input {
                output.push(item(b));
            }

            let (_, end) = b.consume("]");

            TokenTreeBuilder::spanned_square(output, (start, end))
        })
    }

    pub fn spanned_square(input: impl Into<Vec<TokenNode>>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Delimited(Tagged::from_simple_spanned_item(
            DelimitedNode::new(Delimiter::Square, input.into()),
            span,
        ))
    }

    pub fn braced(input: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let (start, _) = b.consume("{ ");
            let mut output = vec![];
            for item in input {
                output.push(item(b));
            }

            let (_, end) = b.consume(" }");

            TokenTreeBuilder::spanned_brace(output, (start, end))
        })
    }

    pub fn spanned_brace(input: impl Into<Vec<TokenNode>>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Delimited(Tagged::from_simple_spanned_item(
            DelimitedNode::new(Delimiter::Brace, input.into()),
            span,
        ))
    }

    pub fn sp() -> CurriedToken {
        Box::new(|b| {
            let (start, end) = b.consume(" ");
            TokenNode::Whitespace(Span::from((start, end)))
        })
    }

    pub fn ws(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&input);
            TokenTreeBuilder::spanned_ws((start, end))
        })
    }

    pub fn spanned_ws(span: impl Into<Span>) -> TokenNode {
        let span = span.into();

        TokenNode::Whitespace(span.into())
    }

    fn consume(&mut self, input: &str) -> (usize, usize) {
        let start = self.pos;
        self.pos += input.len();
        (start, self.pos)
    }
}
