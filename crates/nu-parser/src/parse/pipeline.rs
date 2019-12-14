use crate::{SpannedToken, Token};
use derive_new::new;
use getset::Getters;
use nu_source::{
    b, DebugDocBuilder, HasSpan, IntoSpanned, PrettyDebugWithSource, Span, Spanned, SpannedItem,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Getters, new)]
pub struct Pipeline {
    #[get = "pub"]
    pub(crate) parts: Vec<PipelineElement>,
}

impl IntoSpanned for Pipeline {
    type Output = Spanned<Pipeline>;

    fn into_spanned(self, span: impl Into<Span>) -> Self::Output {
        self.spanned(span.into())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters, new)]
pub struct Tokens {
    pub(crate) tokens: Vec<SpannedToken>,
    pub(crate) span: Span,
}

impl Tokens {
    pub fn iter(&self) -> impl Iterator<Item = &SpannedToken> {
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
    pub fn new(pipe: Option<Span>, tokens: Spanned<Vec<SpannedToken>>) -> PipelineElement {
        PipelineElement {
            pipe,
            tokens: Tokens {
                tokens: tokens.item,
                span: tokens.span,
            },
        }
    }

    pub fn tokens(&self) -> &[SpannedToken] {
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
            self.tokens.iter().map(|token| match token.unspanned() {
                Token::Whitespace => b::blank(),
                _ => token.pretty_debug(source),
            }),
            b::space(),
        )
    }
}
