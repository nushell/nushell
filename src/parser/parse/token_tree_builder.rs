use crate::prelude::*;

use crate::parser::parse::flag::{Flag, FlagKind};
use crate::parser::parse::operator::Operator;
use crate::parser::parse::pipeline::{Pipeline, PipelineElement};
use crate::parser::parse::token_tree::{DelimitedNode, Delimiter, PathNode, TokenNode};
use crate::parser::parse::tokens::{RawNumber, RawToken};
use crate::parser::parse::unit::Unit;
use crate::parser::CallNode;
use derive_new::new;
use uuid::Uuid;

#[derive(new)]
pub struct TokenTreeBuilder {
    #[new(default)]
    pos: usize,

    #[new(default)]
    output: String,

    origin: Uuid,
}

pub type CurriedToken = Box<dyn FnOnce(&mut TokenTreeBuilder) -> TokenNode + 'static>;
pub type CurriedCall = Box<dyn FnOnce(&mut TokenTreeBuilder) -> Tagged<CallNode> + 'static>;

impl TokenTreeBuilder {
    pub fn build(origin: Uuid, block: impl FnOnce(&mut Self) -> TokenNode) -> (TokenNode, String) {
        let mut builder = TokenTreeBuilder::new(origin);
        let node = block(&mut builder);
        (node, builder.output)
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

            let pipe = None;
            let pre_tag = pre.map(|pre| b.consume_tag(&pre));
            let call = call(b);
            let post_tag = post.map(|post| b.consume_tag(&post));

            out.push(PipelineElement::new(pipe, pre_tag, call, post_tag));

            loop {
                match input.next() {
                    None => break,
                    Some((pre, call, post)) => {
                        let pipe = Some(b.consume_tag("|"));
                        let pre_span = pre.map(|pre| b.consume_tag(&pre));
                        let call = call(b);
                        let post_span = post.map(|post| b.consume_tag(&post));

                        out.push(PipelineElement::new(pipe, pre_span, call, post_span));
                    }
                }
            }

            let end = b.pos;

            TokenTreeBuilder::tagged_pipeline((out, None), (start, end, b.origin))
        })
    }

    pub fn tagged_pipeline(
        input: (Vec<PipelineElement>, Option<Tag>),
        tag: impl Into<Tag>,
    ) -> TokenNode {
        TokenNode::Pipeline(Pipeline::new(input.0, input.1.into()).tagged(tag.into()))
    }

    pub fn op(input: impl Into<Operator>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(input.as_str());

            b.pos = end;

            TokenTreeBuilder::tagged_op(input, (start, end, b.origin))
        })
    }

    pub fn tagged_op(input: impl Into<Operator>, tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Operator(input.into().tagged(tag.into()))
    }

    pub fn string(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, _) = b.consume("\"");
            let (inner_start, inner_end) = b.consume(&input);
            let (_, end) = b.consume("\"");
            b.pos = end;

            TokenTreeBuilder::tagged_string(
                (inner_start, inner_end, b.origin),
                (start, end, b.origin),
            )
        })
    }

    pub fn tagged_string(input: impl Into<Tag>, tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Token(RawToken::String(input.into()).tagged(tag.into()))
    }

    pub fn bare(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&input);
            b.pos = end;

            TokenTreeBuilder::tagged_bare((start, end, b.origin))
        })
    }

    pub fn tagged_bare(tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Token(RawToken::Bare.tagged(tag.into()))
    }

    pub fn pattern(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&input);
            b.pos = end;

            TokenTreeBuilder::tagged_pattern((start, end, b.origin))
        })
    }

    pub fn tagged_pattern(input: impl Into<Tag>) -> TokenNode {
        TokenNode::Token(RawToken::GlobPattern.tagged(input.into()))
    }

    pub fn external_word(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&input);
            b.pos = end;

            TokenTreeBuilder::tagged_external_word((start, end, b.origin))
        })
    }

    pub fn tagged_external_word(input: impl Into<Tag>) -> TokenNode {
        TokenNode::Token(RawToken::ExternalWord.tagged(input.into()))
    }

    pub fn tagged_external(input: impl Into<Tag>, tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Token(RawToken::ExternalCommand(input.into()).tagged(tag.into()))
    }

    pub fn int(input: impl Into<BigInt>) -> CurriedToken {
        let int = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&int.to_string());
            b.pos = end;

            TokenTreeBuilder::tagged_number(
                RawNumber::Int((start, end, b.origin).into()),
                (start, end, b.origin),
            )
        })
    }

    pub fn decimal(input: impl Into<BigDecimal>) -> CurriedToken {
        let decimal = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&decimal.to_string());
            b.pos = end;

            TokenTreeBuilder::tagged_number(
                RawNumber::Decimal((start, end, b.origin).into()),
                (start, end, b.origin),
            )
        })
    }

    pub fn tagged_number(input: impl Into<RawNumber>, tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Token(RawToken::Number(input.into()).tagged(tag.into()))
    }

    pub fn size(int: impl Into<i64>, unit: impl Into<Unit>) -> CurriedToken {
        let int = int.into();
        let unit = unit.into();

        Box::new(move |b| {
            let (start_int, end_int) = b.consume(&int.to_string());
            let (_, end_unit) = b.consume(unit.as_str());
            b.pos = end_unit;

            TokenTreeBuilder::tagged_size(
                (RawNumber::Int((start_int, end_int, b.origin).into()), unit),
                (start_int, end_unit, b.origin),
            )
        })
    }

    pub fn tagged_size(
        input: (impl Into<RawNumber>, impl Into<Unit>),
        tag: impl Into<Tag>,
    ) -> TokenNode {
        let (int, unit) = (input.0.into(), input.1.into());

        TokenNode::Token(RawToken::Size(int, unit).tagged(tag.into()))
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

            TokenTreeBuilder::tagged_path((head, output), (start, end, b.origin))
        })
    }

    pub fn tagged_path(input: (TokenNode, Vec<TokenNode>), tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Path(PathNode::new(Box::new(input.0), input.1).tagged(tag.into()))
    }

    pub fn var(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, _) = b.consume("$");
            let (inner_start, end) = b.consume(&input);

            TokenTreeBuilder::tagged_var((inner_start, end, b.origin), (start, end, b.origin))
        })
    }

    pub fn tagged_var(input: impl Into<Tag>, tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Token(RawToken::Variable(input.into()).tagged(tag.into()))
    }

    pub fn flag(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, _) = b.consume("--");
            let (inner_start, end) = b.consume(&input);

            TokenTreeBuilder::tagged_flag((inner_start, end, b.origin), (start, end, b.origin))
        })
    }

    pub fn tagged_flag(input: impl Into<Tag>, tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Flag(Flag::new(FlagKind::Longhand, input.into()).tagged(tag.into()))
    }

    pub fn shorthand(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, _) = b.consume("-");
            let (inner_start, end) = b.consume(&input);

            TokenTreeBuilder::tagged_shorthand((inner_start, end, b.origin), (start, end, b.origin))
        })
    }

    pub fn tagged_shorthand(input: impl Into<Tag>, tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Flag(Flag::new(FlagKind::Shorthand, input.into()).tagged(tag.into()))
    }

    pub fn member(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&input);
            TokenTreeBuilder::tagged_member((start, end, b.origin))
        })
    }

    pub fn tagged_member(tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Member(tag.into())
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

            TokenTreeBuilder::tagged_call(nodes, (start, end, b.origin))
        })
    }

    pub fn tagged_call(input: Vec<TokenNode>, tag: impl Into<Tag>) -> Tagged<CallNode> {
        if input.len() == 0 {
            panic!("BUG: spanned call (TODO)")
        }

        let mut input = input.into_iter();

        let head = input.next().unwrap();
        let tail = input.collect();

        CallNode::new(Box::new(head), tail).tagged(tag.into())
    }

    pub fn parens(input: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let (start, _) = b.consume("(");
            let mut output = vec![];
            for item in input {
                output.push(item(b));
            }

            let (_, end) = b.consume(")");

            TokenTreeBuilder::tagged_parens(output, (start, end, b.origin))
        })
    }

    pub fn tagged_parens(input: impl Into<Vec<TokenNode>>, tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Delimited(DelimitedNode::new(Delimiter::Paren, input.into()).tagged(tag.into()))
    }

    pub fn square(input: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let (start, _) = b.consume("[");
            let mut output = vec![];
            for item in input {
                output.push(item(b));
            }

            let (_, end) = b.consume("]");

            TokenTreeBuilder::tagged_square(output, (start, end, b.origin))
        })
    }

    pub fn tagged_square(input: impl Into<Vec<TokenNode>>, tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Delimited(DelimitedNode::new(Delimiter::Square, input.into()).tagged(tag.into()))
    }

    pub fn braced(input: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let (start, _) = b.consume("{ ");
            let mut output = vec![];
            for item in input {
                output.push(item(b));
            }

            let (_, end) = b.consume(" }");

            TokenTreeBuilder::tagged_brace(output, (start, end, b.origin))
        })
    }

    pub fn tagged_brace(input: impl Into<Vec<TokenNode>>, tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Delimited(DelimitedNode::new(Delimiter::Brace, input.into()).tagged(tag.into()))
    }

    pub fn sp() -> CurriedToken {
        Box::new(|b| {
            let (start, end) = b.consume(" ");
            TokenNode::Whitespace(Tag::from((start, end, b.origin)))
        })
    }

    pub fn ws(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&input);
            TokenTreeBuilder::tagged_ws((start, end, b.origin))
        })
    }

    pub fn tagged_ws(tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Whitespace(tag.into())
    }

    fn consume(&mut self, input: &str) -> (usize, usize) {
        let start = self.pos;
        self.pos += input.len();
        self.output.push_str(input);
        (start, self.pos)
    }

    fn consume_tag(&mut self, input: &str) -> Tag {
        let start = self.pos;
        self.pos += input.len();
        self.output.push_str(input);
        (start, self.pos, self.origin).into()
    }
}
