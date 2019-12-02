use crate::TokenNode;
use derive_new::new;
use getset::Getters;
use nu_source::{b, DebugDocBuilder, HasSpan, PrettyDebugWithSource, Span, Spanned};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Getters, new)]
pub struct Pipeline {
    #[get = "pub"]
    pub(crate) parts: Vec<PipelineElement>,
    pub(crate) span: Span,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters, new)]
pub struct Tokens {
    pub(crate) tokens: Vec<TokenNode>,
    pub(crate) span: Span,
}

impl Tokens {
    pub fn iter(&self) -> impl Iterator<Item = &TokenNode> {
        self.tokens.iter()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters)]
pub struct PipelineElement {
    pub pipe: Option<Span>,
    pub tokens: Tokens,
}

impl HasSpan for PipelineElement {
    fn span(&self) -> Span {
        match self.pipe {
            Option::None => self.tokens.span,
            Option::Some(pipe) => pipe.until(self.tokens.span),
        }
    }
}

impl PipelineElement {
    pub fn new(pipe: Option<Span>, tokens: Spanned<Vec<TokenNode>>) -> PipelineElement {
        PipelineElement {
            pipe,
            tokens: Tokens {
                tokens: tokens.item,
                span: tokens.span,
            },
        }
    }

    pub fn tokens(&self) -> &[TokenNode] {
        &self.tokens.tokens
    }
}

impl PrettyDebugWithSource for Pipeline {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::intersperse(
            self.parts.iter().map(|token| token.pretty_debug(source)),
            b::operator(" | "),
        )
    }
}

impl PrettyDebugWithSource for PipelineElement {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::intersperse(
            self.tokens.iter().map(|token| match token {
                TokenNode::Whitespace(_) => b::blank(),
                token => token.pretty_debug(source),
            }),
            b::space(),
        )
    }
}
