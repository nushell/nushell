use crate::parse::call_node::CallNode;
use crate::parse::comment::Comment;
use crate::parse::flag::{Flag, FlagKind};
use crate::parse::operator::{CompareOperator, EvaluationOperator};
use crate::parse::pipeline::{Pipeline, PipelineElement};
use crate::parse::token_tree::{DelimitedNode, Delimiter, TokenNode};
use crate::parse::tokens::{RawNumber, UnspannedToken};
use bigdecimal::BigDecimal;
use nu_source::{Span, Spanned, SpannedItem};
use num_bigint::BigInt;

#[derive(Default)]
pub struct TokenTreeBuilder {
    pos: usize,
    output: String,
}

impl TokenTreeBuilder {
    pub fn new() -> Self {
        Default::default()
    }
}

pub type CurriedToken = Box<dyn FnOnce(&mut TokenTreeBuilder) -> TokenNode + 'static>;
pub type CurriedCall = Box<dyn FnOnce(&mut TokenTreeBuilder) -> Spanned<CallNode> + 'static>;

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

            let mut out: Vec<PipelineElement> = vec![];

            let mut input = input.into_iter().peekable();
            let head = input
                .next()
                .expect("A pipeline must contain at least one element");

            let pipe = None;
            let head = b.build_spanned(|b| head.into_iter().map(|node| node(b)).collect());

            out.push(PipelineElement::new(pipe, head));

            loop {
                match input.next() {
                    None => break,
                    Some(node) => {
                        let pipe = Some(b.consume_span("|"));
                        let node =
                            b.build_spanned(|b| node.into_iter().map(|node| node(b)).collect());

                        out.push(PipelineElement::new(pipe, node));
                    }
                }
            }

            let end = b.pos;

            TokenTreeBuilder::spanned_pipeline(out, Span::new(start, end))
        })
    }

    pub fn spanned_pipeline(input: Vec<PipelineElement>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Pipeline(Pipeline::new(input, span.into()))
    }

    pub fn token_list(input: Vec<CurriedToken>) -> CurriedToken {
        Box::new(move |b| {
            let start = b.pos;
            let tokens = input.into_iter().map(|i| i(b)).collect();
            let end = b.pos;

            TokenTreeBuilder::spanned_token_list(tokens, Span::new(start, end))
        })
    }

    pub fn spanned_token_list(input: Vec<TokenNode>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Nodes(input.spanned(span.into()))
    }

    pub fn op(input: impl Into<CompareOperator>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(input.as_str());

            b.pos = end;

            TokenTreeBuilder::spanned_cmp_op(input, Span::new(start, end))
        })
    }

    pub fn spanned_cmp_op(input: impl Into<CompareOperator>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Token(UnspannedToken::CompareOperator(input.into()).into_token(span))
    }

    pub fn dot() -> CurriedToken {
        Box::new(move |b| {
            let (start, end) = b.consume(".");

            b.pos = end;

            TokenTreeBuilder::spanned_eval_op(".", Span::new(start, end))
        })
    }

    pub fn dotdot() -> CurriedToken {
        Box::new(move |b| {
            let (start, end) = b.consume("..");

            b.pos = end;

            TokenTreeBuilder::spanned_eval_op("..", Span::new(start, end))
        })
    }

    pub fn spanned_eval_op(
        input: impl Into<EvaluationOperator>,
        span: impl Into<Span>,
    ) -> TokenNode {
        TokenNode::Token(UnspannedToken::EvaluationOperator(input.into()).into_token(span))
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
        TokenNode::Token(UnspannedToken::String(input.into()).into_token(span))
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
        TokenNode::Token(UnspannedToken::Bare.into_token(span))
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
        TokenNode::Token(UnspannedToken::GlobPattern.into_token(input))
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
        TokenNode::Token(UnspannedToken::ExternalWord.into_token(input))
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
        TokenNode::Token(UnspannedToken::ExternalCommand(inner.into()).into_token(outer))
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
        TokenNode::Token(UnspannedToken::Number(input.into()).into_token(span))
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
        TokenNode::Token(UnspannedToken::Variable(input.into()).into_token(span))
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
        TokenNode::Flag(Flag::new(FlagKind::Longhand, input.into(), span.into()))
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
        TokenNode::Flag(Flag::new(FlagKind::Shorthand, input.into(), span.into()))
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

            TokenTreeBuilder::spanned_call(nodes, Span::new(start, end))
        })
    }

    pub fn spanned_call(input: Vec<TokenNode>, span: impl Into<Span>) -> Spanned<CallNode> {
        if input.is_empty() {
            panic!("BUG: spanned call (TODO)")
        }

        let mut input = input.into_iter();

        if let Some(head) = input.next() {
            let tail = input.collect();

            CallNode::new(Box::new(head), tail).spanned(span.into())
        } else {
            unreachable!("Internal error: spanned_call failed")
        }
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

    pub fn sep(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let (start, end) = b.consume(&input);
            TokenTreeBuilder::spanned_sep(Span::new(start, end))
        })
    }

    pub fn spanned_sep(span: impl Into<Span>) -> TokenNode {
        TokenNode::Separator(span.into())
    }

    pub fn comment(input: impl Into<String>) -> CurriedToken {
        let input = input.into();

        Box::new(move |b| {
            let outer_start = b.pos;
            b.consume("#");
            let (start, end) = b.consume(&input);
            let outer_end = b.pos;

            TokenTreeBuilder::spanned_comment((start, end), (outer_start, outer_end))
        })
    }

    pub fn spanned_comment(input: impl Into<Span>, span: impl Into<Span>) -> TokenNode {
        TokenNode::Comment(Comment::line(input, span))
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
