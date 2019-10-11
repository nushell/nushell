use crate::prelude::*;

use crate::parser::parse::flag::{Flag, FlagKind};
use crate::parser::parse::operator::Operator;
use crate::parser::parse::pipeline::{Pipeline, PipelineElement};
use crate::parser::parse::token_tree::{DelimitedNode, Delimiter, TokenNode};
use crate::parser::parse::tokens::{RawNumber, RawToken};
use crate::parser::CallNode;
use derive_new::new;
use uuid::Uuid;

#[derive(new)]
pub struct TokenTreeBuilder {
    #[new(default)]
    pos: usize,

    #[new(default)]
    output: String,

    anchor: Uuid,
}

pub type CurriedToken = Box<dyn FnOnce(&mut TokenTreeBuilder) -> TokenNode + 'static>;
pub type CurriedCall = Box<dyn FnOnce(&mut TokenTreeBuilder) -> Tagged<CallNode> + 'static>;

impl TokenTreeBuilder {
    pub fn build(anchor: Uuid, block: impl FnOnce(&mut Self) -> TokenNode) -> (TokenNode, String) {
        let mut builder = TokenTreeBuilder::new(anchor);
        let node = block(&mut builder);
        (node, builder.output)
    }

    fn build_tagged<T>(&mut self, callback: impl FnOnce(&mut TokenTreeBuilder) -> T) -> Tagged<T> {
        let start = self.pos;
        let ret = callback(self);
        let end = self.pos;

        ret.tagged((start, end, self.anchor))
    }

    pub fn pipeline(input: Vec<Vec<CurriedToken>>) -> CurriedToken {
        Box::new(move |b| {
            let start = b.pos;

            let mut out: Vec<Tagged<PipelineElement>> = vec![];

            let mut input = input.into_iter().peekable();
            let head = input
                .next()
                .expect("A pipeline must contain at least one element");

            let pipe = None;
            let head = b.build_tagged(|b| head.into_iter().map(|node| node(b)).collect());

            let head_tag: Tag = head.tag;
            out.push(PipelineElement::new(pipe, head).tagged(head_tag));

            loop {
                match input.next() {
                    None => break,
                    Some(node) => {
                        let start = b.pos;
                        let pipe = Some(b.consume_tag("|"));
                        let node =
                            b.build_tagged(|b| node.into_iter().map(|node| node(b)).collect());
                        let end = b.pos;

                        out.push(PipelineElement::new(pipe, node).tagged((start, end, b.anchor)));
                    }
                }
            }

            let end = b.pos;

            TokenTreeBuilder::tagged_pipeline(out, (start, end, b.anchor))
        })
    }

    pub fn tagged_pipeline(input: Vec<Tagged<PipelineElement>>, tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Pipeline(Pipeline::new(input).tagged(tag.into()))
    }

    pub fn token_list(input: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let start = b.pos;
            let tokens = input.into_iter().map(|i| i(b)).collect();
            let end = b.pos;

            TokenTreeBuilder::tagged_token_list(tokens, (start, end, b.anchor))
        })
    }

    pub fn tagged_token_list(input: Vec<TokenNode>, tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Nodes(input.tagged(tag))
    }

    pub fn op(input: impl Into<Operator>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(input.as_str());

            b.pos = end;

            TokenTreeBuilder::tagged_op(input, (start, end, b.anchor))
        })
    }

    pub fn tagged_op(input: impl Into<Operator>, tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Token(RawToken::Operator(input.into()).tagged(tag.into()))
    }

    pub fn string(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, _) = b.consume("\"");
            let (inner_start, inner_end) = b.consume(&input);
            let (_, end) = b.consume("\"");
            b.pos = end;

            TokenTreeBuilder::tagged_string(
                (inner_start, inner_end, b.anchor),
                (start, end, b.anchor),
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

            TokenTreeBuilder::tagged_bare((start, end, b.anchor))
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

            TokenTreeBuilder::tagged_pattern((start, end, b.anchor))
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

            TokenTreeBuilder::tagged_external_word((start, end, b.anchor))
        })
    }

    pub fn tagged_external_word(input: impl Into<Tag>) -> TokenNode {
        TokenNode::Token(RawToken::ExternalWord.tagged(input.into()))
    }

    pub fn external_command(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (outer_start, _) = b.consume("^");
            let (inner_start, end) = b.consume(&input);
            b.pos = end;

            TokenTreeBuilder::tagged_external_command(
                (inner_start, end, b.anchor),
                (outer_start, end, b.anchor),
            )
        })
    }

    pub fn tagged_external_command(inner: impl Into<Tag>, outer: impl Into<Tag>) -> TokenNode {
        TokenNode::Token(RawToken::ExternalCommand(inner.into()).tagged(outer.into()))
    }

    pub fn int(input: impl Into<BigInt>) -> CurriedToken {
        let int = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&int.to_string());
            b.pos = end;

            TokenTreeBuilder::tagged_number(
                RawNumber::Int((start, end, b.anchor).into()),
                (start, end, b.anchor),
            )
        })
    }

    pub fn decimal(input: impl Into<BigDecimal>) -> CurriedToken {
        let decimal = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&decimal.to_string());
            b.pos = end;

            TokenTreeBuilder::tagged_number(
                RawNumber::Decimal((start, end, b.anchor).into()),
                (start, end, b.anchor),
            )
        })
    }

    pub fn tagged_number(input: impl Into<RawNumber>, tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Token(RawToken::Number(input.into()).tagged(tag.into()))
    }

    pub fn var(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, _) = b.consume("$");
            let (inner_start, end) = b.consume(&input);

            TokenTreeBuilder::tagged_var((inner_start, end, b.anchor), (start, end, b.anchor))
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

            TokenTreeBuilder::tagged_flag((inner_start, end, b.anchor), (start, end, b.anchor))
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

            TokenTreeBuilder::tagged_shorthand((inner_start, end, b.anchor), (start, end, b.anchor))
        })
    }

    pub fn tagged_shorthand(input: impl Into<Tag>, tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Flag(Flag::new(FlagKind::Shorthand, input.into()).tagged(tag.into()))
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

            TokenTreeBuilder::tagged_call(nodes, (start, end, b.anchor))
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

    fn consume_delimiter(
        &mut self,
        input: Vec<CurriedToken>,
        _open: &str,
        _close: &str,
    ) -> (Tag, Tag, Tag, Vec<TokenNode>) {
        let (start_open_paren, end_open_paren) = self.consume("(");
        let mut output = vec![];
        for item in input {
            output.push(item(self));
        }

        let (start_close_paren, end_close_paren) = self.consume(")");

        let open = Tag::from((start_open_paren, end_open_paren, self.anchor));
        let close = Tag::from((start_close_paren, end_close_paren, self.anchor));
        let whole = Tag::from((start_open_paren, end_close_paren, self.anchor));

        (open, close, whole, output)
    }

    pub fn parens(input: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let (open, close, whole, output) = b.consume_delimiter(input, "(", ")");

            TokenTreeBuilder::tagged_parens(output, (open, close), whole)
        })
    }

    pub fn tagged_parens(
        input: impl Into<Vec<TokenNode>>,
        tags: (Tag, Tag),
        tag: impl Into<Tag>,
    ) -> TokenNode {
        TokenNode::Delimited(
            DelimitedNode::new(Delimiter::Paren, tags, input.into()).tagged(tag.into()),
        )
    }

    pub fn square(input: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let (open, close, whole, tokens) = b.consume_delimiter(input, "[", "]");

            TokenTreeBuilder::tagged_square(tokens, (open, close), whole)
        })
    }

    pub fn tagged_square(
        input: impl Into<Vec<TokenNode>>,
        tags: (Tag, Tag),
        tag: impl Into<Tag>,
    ) -> TokenNode {
        TokenNode::Delimited(
            DelimitedNode::new(Delimiter::Square, tags, input.into()).tagged(tag.into()),
        )
    }

    pub fn braced(input: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let (open, close, whole, tokens) = b.consume_delimiter(input, "{", "}");

            TokenTreeBuilder::tagged_brace(tokens, (open, close), whole)
        })
    }

    pub fn tagged_brace(
        input: impl Into<Vec<TokenNode>>,
        tags: (Tag, Tag),
        tag: impl Into<Tag>,
    ) -> TokenNode {
        TokenNode::Delimited(
            DelimitedNode::new(Delimiter::Brace, tags, input.into()).tagged(tag.into()),
        )
    }

    pub fn sp() -> CurriedToken {
        Box::new(|b| {
            let (start, end) = b.consume(" ");
            TokenNode::Whitespace(Tag::from((start, end, b.anchor)))
        })
    }

    pub fn ws(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&input);
            TokenTreeBuilder::tagged_ws((start, end, b.anchor))
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
        (start, self.pos, self.anchor).into()
    }
}
