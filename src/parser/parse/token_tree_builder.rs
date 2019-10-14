use crate::prelude::*;

use crate::parser::parse::flag::{Flag, FlagKind};
use crate::parser::parse::operator::Operator;
use crate::parser::parse::pipeline::{Pipeline, PipelineElement};
use crate::parser::parse::token_tree::{DelimitedNode, Delimiter, TokenNode};
use crate::parser::parse::tokens::{RawNumber, RawToken};
use crate::parser::CallNode;
use derive_new::new;

#[derive(new)]
pub struct TokenTreeBuilder {
    #[new(default)]
    pos: usize,

    #[new(default)]
    output: String,
}

pub type CurriedToken = Box<dyn FnOnce(&mut TokenTreeBuilder) -> TokenNode + 'static>;
pub type CurriedCall = Box<dyn FnOnce(&mut TokenTreeBuilder) -> Tagged<CallNode> + 'static>;

impl TokenTreeBuilder {
    pub fn build(block: impl FnOnce(&mut Self) -> TokenNode) -> (TokenNode, String) {
        let mut builder = TokenTreeBuilder::new();
        let node = block(&mut builder);
        (node, builder.output)
    }

    fn build_spanned<T>(
        &mut self,
        callback: impl FnOnce(&mut TokenTreeBuilder) -> T,
    ) -> Spanned<T> {
        let start = self.pos;
        let ret = callback(self);
        let end = self.pos;

        ret.spanned(Span::new(start, end))
    }

    pub fn pipeline(input: Vec<Vec<CurriedToken>>) -> CurriedToken {
        Box::new(move |b| {
            let start = b.pos;

            let mut out: Vec<Spanned<PipelineElement>> = vec![];

            let mut input = input.into_iter().peekable();
            let head = input
                .next()
                .expect("A pipeline must contain at least one element");

            let pipe = None;
            let head = b.build_spanned(|b| head.into_iter().map(|node| node(b)).collect());

            let head_span: Span = head.span;
            out.push(PipelineElement::new(pipe, head).spanned(head_span));

            loop {
                match input.next() {
                    None => break,
                    Some(node) => {
                        let start = b.pos;
                        let pipe = Some(b.consume_span("|"));
                        let node =
                            b.build_spanned(|b| node.into_iter().map(|node| node(b)).collect());
                        let end = b.pos;

                        out.push(PipelineElement::new(pipe, node).spanned(Span::new(start, end)));
                    }
                }
            }

            let end = b.pos;

            TokenTreeBuilder::spanned_pipeline(out, Span::new(start, end))
        })
    }

    pub fn spanned_pipeline(
        input: Vec<Spanned<PipelineElement>>,
        span: impl Into<Span>,
    ) -> TokenNode {
        TokenNode::Pipeline(Pipeline::new(input).spanned(span))
    }

    pub fn token_list(input: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let start = b.pos;
            let tokens = input.into_iter().map(|i| i(b)).collect();
            let end = b.pos;

            TokenTreeBuilder::tagged_token_list(tokens, (start, end, None))
        })
    }

    pub fn tagged_token_list(input: Vec<TokenNode>, tag: impl Into<Tag>) -> TokenNode {
        TokenNode::Nodes(input.spanned(tag.into().span))
    }

    pub fn op(input: impl Into<Operator>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(input.as_str());

            b.pos = end;

            TokenTreeBuilder::spanned_op(input, Span::new(start, end))
        })
    }

    pub fn spanned_op(input: impl Into<Operator>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(RawToken::Operator(input.into()).spanned(span.into()))
    }

    pub fn string(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, _) = b.consume("\"");
            let (inner_start, inner_end) = b.consume(&input);
            let (_, end) = b.consume("\"");
            b.pos = end;

            TokenTreeBuilder::spanned_string(
                Span::new(inner_start, inner_end),
                Span::new(start, end),
            )
        })
    }

    pub fn spanned_string(input: impl Into<Span>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(RawToken::String(input.into()).spanned(span.into()))
    }

    pub fn bare(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&input);
            b.pos = end;

            TokenTreeBuilder::spanned_bare(Span::new(start, end))
        })
    }

    pub fn spanned_bare(span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(RawToken::Bare.spanned(span))
    }

    pub fn pattern(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&input);
            b.pos = end;

            TokenTreeBuilder::spanned_pattern(Span::new(start, end))
        })
    }

    pub fn spanned_pattern(input: impl Into<Span>) -> TokenNode {
        TokenNode::Token(RawToken::GlobPattern.spanned(input.into()))
    }

    pub fn external_word(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&input);
            b.pos = end;

            TokenTreeBuilder::spanned_external_word(Span::new(start, end))
        })
    }

    pub fn spanned_external_word(input: impl Into<Span>) -> TokenNode {
        TokenNode::Token(RawToken::ExternalWord.spanned(input.into()))
    }

    pub fn external_command(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (outer_start, _) = b.consume("^");
            let (inner_start, end) = b.consume(&input);
            b.pos = end;

            TokenTreeBuilder::spanned_external_command(
                Span::new(inner_start, end),
                Span::new(outer_start, end),
            )
        })
    }

    pub fn spanned_external_command(inner: impl Into<Span>, outer: impl Into<Span>) -> TokenNode {
        TokenNode::Token(RawToken::ExternalCommand(inner.into()).spanned(outer.into()))
    }

    pub fn int(input: impl Into<BigInt>) -> CurriedToken {
        let int = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&int.to_string());
            b.pos = end;

            TokenTreeBuilder::spanned_number(
                RawNumber::Int(Span::new(start, end)),
                Span::new(start, end),
            )
        })
    }

    pub fn decimal(input: impl Into<BigDecimal>) -> CurriedToken {
        let decimal = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&decimal.to_string());
            b.pos = end;

            TokenTreeBuilder::spanned_number(
                RawNumber::Decimal(Span::new(start, end)),
                Span::new(start, end),
            )
        })
    }

    pub fn spanned_number(input: impl Into<RawNumber>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(RawToken::Number(input.into()).spanned(span.into()))
    }

    pub fn var(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, _) = b.consume("$");
            let (inner_start, end) = b.consume(&input);

            TokenTreeBuilder::spanned_var(Span::new(inner_start, end), Span::new(start, end))
        })
    }

    pub fn spanned_var(input: impl Into<Span>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(RawToken::Variable(input.into()).spanned(span.into()))
    }

    pub fn flag(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, _) = b.consume("--");
            let (inner_start, end) = b.consume(&input);

            TokenTreeBuilder::spanned_flag(Span::new(inner_start, end), Span::new(start, end))
        })
    }

    pub fn spanned_flag(input: impl Into<Span>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Flag(Flag::new(FlagKind::Longhand, input.into()).spanned(span.into()))
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
        TokenNode::Flag(Flag::new(FlagKind::Shorthand, input.into()).spanned(span.into()))
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

            TokenTreeBuilder::tagged_call(nodes, (start, end, None))
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
    ) -> (Span, Span, Span, Vec<TokenNode>) {
        let (start_open_paren, end_open_paren) = self.consume("(");
        let mut output = vec![];
        for item in input {
            output.push(item(self));
        }

        let (start_close_paren, end_close_paren) = self.consume(")");

        let open = Span::new(start_open_paren, end_open_paren);
        let close = Span::new(start_close_paren, end_close_paren);
        let whole = Span::new(start_open_paren, end_close_paren);

        (open, close, whole, output)
    }

    pub fn parens(input: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let (open, close, whole, output) = b.consume_delimiter(input, "(", ")");

            TokenTreeBuilder::spanned_parens(output, (open, close), whole)
        })
    }

    pub fn spanned_parens(
        input: impl Into<Vec<TokenNode>>,
        spans: (Span, Span),
        span: impl Into<Span>,
    ) -> TokenNode {
        TokenNode::Delimited(
            DelimitedNode::new(Delimiter::Paren, spans, input.into()).spanned(span.into()),
        )
    }

    pub fn square(input: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let (open, close, whole, tokens) = b.consume_delimiter(input, "[", "]");

            TokenTreeBuilder::spanned_square(tokens, (open, close), whole)
        })
    }

    pub fn spanned_square(
        input: impl Into<Vec<TokenNode>>,
        spans: (Span, Span),
        span: impl Into<Span>,
    ) -> TokenNode {
        TokenNode::Delimited(
            DelimitedNode::new(Delimiter::Square, spans, input.into()).spanned(span.into()),
        )
    }

    pub fn braced(input: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let (open, close, whole, tokens) = b.consume_delimiter(input, "{", "}");

            TokenTreeBuilder::spanned_brace(tokens, (open, close), whole)
        })
    }

    pub fn spanned_brace(
        input: impl Into<Vec<TokenNode>>,
        spans: (Span, Span),
        span: impl Into<Span>,
    ) -> TokenNode {
        TokenNode::Delimited(
            DelimitedNode::new(Delimiter::Brace, spans, input.into()).spanned(span.into()),
        )
    }

    pub fn sp() -> CurriedToken {
        Box::new(|b| {
            let (start, end) = b.consume(" ");
            TokenNode::Whitespace(Span::new(start, end))
        })
    }

    pub fn ws(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&input);
            TokenTreeBuilder::spanned_ws(Span::new(start, end))
        })
    }

    pub fn spanned_ws(span: impl Into<Span>) -> TokenNode {
        TokenNode::Whitespace(span.into())
    }

    fn consume(&mut self, input: &str) -> (usize, usize) {
        let start = self.pos;
        self.pos += input.len();
        self.output.push_str(input);
        (start, self.pos)
    }

    fn consume_span(&mut self, input: &str) -> Span {
        let start = self.pos;
        self.pos += input.len();
        self.output.push_str(input);
        Span::new(start, self.pos)
    }
}
