use crate::parser::parse2::operator::*;
use crate::parser::parse2::span::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RawToken {
    Integer(i64),
    Operator(Operator),
    String(Span),
}

pub type Token = Spanned<RawToken>;
