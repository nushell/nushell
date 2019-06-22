use crate::parser::parse2::flag::*;
use crate::parser::parse2::operator::*;
use crate::parser::parse2::span::*;
use crate::parser::parse2::unit::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RawToken {
    Integer(i64),
    Size(i64, Unit),
    String(Span),
    Variable(Span),
    Bare,
}

pub type Token = Spanned<RawToken>;

impl Token {
    pub fn to_semantic_token(&self) -> Option<SemanticToken> {
        let semantic_token = match self.item {
            RawToken::Integer(int) => RawSemanticToken::Integer(int),
            RawToken::Size(int, unit) => RawSemanticToken::Size(int, unit),
            RawToken::String(span) => RawSemanticToken::String(span),
            RawToken::Variable(span) => RawSemanticToken::Variable(span),
            RawToken::Bare => RawSemanticToken::Bare,
        };

        Some(Spanned::from_item(semantic_token, self.span))
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RawSemanticToken {
    Integer(i64),
    Size(i64, Unit),
    String(Span),
    Variable(Span),
    Bare,
}

pub type SemanticToken = Spanned<RawSemanticToken>;
