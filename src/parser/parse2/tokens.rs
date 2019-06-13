use crate::parser::parse2::flag::*;
use crate::parser::parse2::operator::*;
use crate::parser::parse2::span::*;
use crate::parser::parse2::unit::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RawToken {
    Integer(i64),
    Size(i64, Unit),
    Operator(Operator),
    String(Span),
    Variable(Span),
    Identifier,
    Bare,
    Flag(Flag, Span),
}

pub type Token = Spanned<RawToken>;
