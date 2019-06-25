use crate::parser::parse::span::*;
use crate::parser::parse::unit::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RawToken {
    Integer(i64),
    Size(i64, Unit),
    String(Span),
    Variable(Span),
    Bare,
}

pub type Token = Spanned<RawToken>;
