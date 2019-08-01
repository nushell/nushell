use crate::parser::parse::unit::*;
use crate::{Span, Tagged, Text};
use std::fmt;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RawToken {
    Integer(i64),
    Size(i64, Unit),
    String(Span),
    Variable(Span),
    Bare,
}

impl RawToken {
    pub fn type_name(&self) -> &'static str {
        match self {
            RawToken::Integer(_) => "Integer",
            RawToken::Size(..) => "Size",
            RawToken::String(_) => "String",
            RawToken::Variable(_) => "Variable",
            RawToken::Bare => "String",
        }
    }
}

pub type Token = Tagged<RawToken>;

impl Token {
    pub fn debug(&self, source: &'a Text) -> DebugToken<'a> {
        DebugToken {
            node: *self,
            source,
        }
    }
}

pub struct DebugToken<'a> {
    node: Token,
    source: &'a Text,
}

impl fmt::Debug for DebugToken<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.node.span().slice(self.source))
    }
}
