use crate::parser::Operator;
use crate::prelude::*;
use crate::{Tagged, Text};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RawToken {
    Number(RawNumber),
    Operator(Operator),
    String(Tag),
    Variable(Tag),
    ExternalCommand(Tag),
    ExternalWord,
    GlobPattern,
    Bare,
}

impl RawToken {
    pub fn type_name(&self) -> &'static str {
        match self {
            RawToken::Number(_) => "Number",
            RawToken::Operator(..) => "operator",
            RawToken::String(_) => "String",
            RawToken::Variable(_) => "variable",
            RawToken::ExternalCommand(_) => "external command",
            RawToken::ExternalWord => "external word",
            RawToken::GlobPattern => "glob pattern",
            RawToken::Bare => "String",
        }
    }
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

pub type Token = Tagged<RawToken>;

impl Token {
    pub fn debug<'a>(&self, source: &'a Text) -> DebugToken<'a> {
        DebugToken {
            node: *self,
            source,
        }
    }

    pub fn extract_number(&self) -> Option<Tagged<RawNumber>> {
        match self.item {
            RawToken::Number(number) => Some((number).tagged(self.tag)),
            _ => None,
        }
    }

    pub fn extract_int(&self) -> Option<(Tag, Tag)> {
        match self.item {
            RawToken::Number(RawNumber::Int(int)) => Some((int, self.tag)),
            _ => None,
        }
    }

    pub fn extract_decimal(&self) -> Option<(Tag, Tag)> {
        match self.item {
            RawToken::Number(RawNumber::Decimal(decimal)) => Some((decimal, self.tag)),
            _ => None,
        }
    }

    pub fn extract_operator(&self) -> Option<Tagged<Operator>> {
        match self.item {
            RawToken::Operator(operator) => Some(operator.tagged(self.tag)),
            _ => None,
        }
    }

    pub fn extract_string(&self) -> Option<(Tag, Tag)> {
        match self.item {
            RawToken::String(tag) => Some((tag, self.tag)),
            _ => None,
        }
    }

    pub fn extract_variable(&self) -> Option<(Tag, Tag)> {
        match self.item {
            RawToken::Variable(tag) => Some((tag, self.tag)),
            _ => None,
        }
    }

    pub fn extract_external_command(&self) -> Option<(Tag, Tag)> {
        match self.item {
            RawToken::ExternalCommand(tag) => Some((tag, self.tag)),
            _ => None,
        }
    }

    pub fn extract_external_word(&self) -> Option<Tag> {
        match self.item {
            RawToken::ExternalWord => Some(self.tag),
            _ => None,
        }
    }

    pub fn extract_glob_pattern(&self) -> Option<Tag> {
        match self.item {
            RawToken::GlobPattern => Some(self.tag),
            _ => None,
        }
    }

    pub fn extract_bare(&self) -> Option<Tag> {
        match self.item {
            RawToken::Bare => Some(self.tag),
            _ => None,
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
