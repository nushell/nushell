use crate::parser::parse::unit::*;
use crate::parser::Operator;
use crate::prelude::*;
use crate::{Tagged, Text};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RawToken {
    Number(RawNumber),
    Operator(Operator),
    Size(RawNumber, Unit),
    String(Tag),
    Variable(Tag),
    ExternalCommand(Tag),
    ExternalWord,
    GlobPattern,
    Bare,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RawNumber {
    Int(Tag),
    Decimal(Tag),
}

impl RawNumber {
    pub fn int(tag: impl Into<Tag>) -> Tagged<RawNumber> {
        let tag = tag.into();

        RawNumber::Int(tag).tagged(tag)
    }

    pub fn decimal(tag: impl Into<Tag>) -> Tagged<RawNumber> {
        let tag = tag.into();

        RawNumber::Decimal(tag).tagged(tag)
    }

    pub(crate) fn to_number(self, source: &Text) -> Number {
        match self {
            RawNumber::Int(tag) => Number::Int(BigInt::from_str(tag.slice(source)).unwrap()),
            RawNumber::Decimal(tag) => {
                Number::Decimal(BigDecimal::from_str(tag.slice(source)).unwrap())
            }
        }
    }
}

impl RawToken {
    pub fn type_name(&self) -> &'static str {
        match self {
            RawToken::Number(_) => "Number",
            RawToken::Operator(..) => "operator",
            RawToken::Size(..) => "Size",
            RawToken::String(_) => "String",
            RawToken::Variable(_) => "variable",
            RawToken::ExternalCommand(_) => "external command",
            RawToken::ExternalWord => "external word",
            RawToken::GlobPattern => "glob pattern",
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
        write!(f, "{}", self.node.tag().slice(self.source))
    }
}
