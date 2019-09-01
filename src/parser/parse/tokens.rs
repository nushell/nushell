use crate::parser::parse::unit::*;
use crate::prelude::*;
use crate::{Span, Tagged, Text};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RawToken {
    Number(RawNumber),
    Size(RawNumber, Unit),
    String(Span),
    Variable(Span),
    External(Span),
    Bare,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RawNumber {
    Int(Span),
    Decimal(Span),
}

impl RawNumber {
    pub fn int(span: impl Into<Span>) -> Tagged<RawNumber> {
        let span = span.into();

        RawNumber::Int(span).tagged(span)
    }

    pub fn decimal(span: impl Into<Span>) -> Tagged<RawNumber> {
        let span = span.into();

        RawNumber::Decimal(span).tagged(span)
    }

    pub(crate) fn to_number(self, source: &Text) -> Number {
        match self {
            RawNumber::Int(span) => Number::Int(BigInt::from_str(span.slice(source)).unwrap()),
            RawNumber::Decimal(span) => {
                Number::Decimal(BigDecimal::from_str(span.slice(source)).unwrap())
            }
        }
    }
}

impl RawToken {
    pub fn type_name(&self) -> &'static str {
        match self {
            RawToken::Number(_) => "Number",
            RawToken::Size(..) => "Size",
            RawToken::String(_) => "String",
            RawToken::Variable(_) => "Variable",
            RawToken::External(_) => "External",
            RawToken::Bare => "String",
        }
    }
}

pub type Token = Tagged<RawToken>;

impl Token {
    pub fn debug<'a>(&self, source: &'a Text) -> DebugToken<'a> {
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

impl fmt::Debug for DebugToken<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.node.span().slice(self.source))
    }
}
