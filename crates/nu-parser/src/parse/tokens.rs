use crate::parse::parser::Number;
use crate::Operator;
use bigdecimal::BigDecimal;
use nu_protocol::ShellTypeName;
use nu_source::{
    b, DebugDocBuilder, HasSpan, PrettyDebug, PrettyDebugWithSource, Span, Spanned, SpannedItem,
    Text,
};
use num_bigint::BigInt;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum UnspannedToken {
    Number(RawNumber),
    Operator(Operator),
    String(Span),
    Variable(Span),
    ExternalCommand(Span),
    ExternalWord,
    GlobPattern,
    Bare,
}

impl UnspannedToken {
    pub fn into_token(self, span: impl Into<Span>) -> Token {
        Token {
            unspanned: self,
            span: span.into(),
        }
    }
}

impl ShellTypeName for UnspannedToken {
    fn type_name(&self) -> &'static str {
        match self {
            UnspannedToken::Number(_) => "number",
            UnspannedToken::Operator(..) => "operator",
            UnspannedToken::String(_) => "string",
            UnspannedToken::Variable(_) => "variable",
            UnspannedToken::ExternalCommand(_) => "syntax error",
            UnspannedToken::ExternalWord => "syntax error",
            UnspannedToken::GlobPattern => "glob pattern",
            UnspannedToken::Bare => "string",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RawNumber {
    Int(Span),
    Decimal(Span),
}

impl HasSpan for RawNumber {
    fn span(&self) -> Span {
        match self {
            RawNumber::Int(span) => *span,
            RawNumber::Decimal(span) => *span,
        }
    }
}

impl PrettyDebugWithSource for RawNumber {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self {
            RawNumber::Int(span) => b::primitive(span.slice(source)),
            RawNumber::Decimal(span) => b::primitive(span.slice(source)),
        }
    }
}

impl RawNumber {
    pub fn int(span: impl Into<Span>) -> RawNumber {
        let span = span.into();

        RawNumber::Int(span)
    }

    pub fn decimal(span: impl Into<Span>) -> RawNumber {
        let span = span.into();

        RawNumber::Decimal(span)
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Token {
    pub unspanned: UnspannedToken,
    pub span: Span,
}

impl std::ops::Deref for Token {
    type Target = UnspannedToken;

    fn deref(&self) -> &UnspannedToken {
        &self.unspanned
    }
}

impl PrettyDebugWithSource for Token {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self.unspanned {
            UnspannedToken::Number(number) => number.pretty_debug(source),
            UnspannedToken::Operator(operator) => operator.pretty(),
            UnspannedToken::String(_) => b::primitive(self.span.slice(source)),
            UnspannedToken::Variable(_) => b::var(self.span.slice(source)),
            UnspannedToken::ExternalCommand(_) => b::primitive(self.span.slice(source)),
            UnspannedToken::ExternalWord => {
                b::typed("external", b::description(self.span.slice(source)))
            }
            UnspannedToken::GlobPattern => {
                b::typed("pattern", b::description(self.span.slice(source)))
            }
            UnspannedToken::Bare => b::primitive(self.span.slice(source)),
        }
    }
}

impl Token {
    pub fn debug<'a>(&self, source: &'a Text) -> DebugToken<'a> {
        DebugToken {
            node: *self,
            source,
        }
    }

    pub fn extract_number(&self) -> Option<RawNumber> {
        match self.unspanned {
            UnspannedToken::Number(number) => Some(number),
            _ => None,
        }
    }

    pub fn extract_int(&self) -> Option<(Span, Span)> {
        match self.unspanned {
            UnspannedToken::Number(RawNumber::Int(int)) => Some((int, self.span)),
            _ => None,
        }
    }

    pub fn extract_decimal(&self) -> Option<(Span, Span)> {
        match self.unspanned {
            UnspannedToken::Number(RawNumber::Decimal(decimal)) => Some((decimal, self.span)),
            _ => None,
        }
    }

    pub fn extract_operator(&self) -> Option<Spanned<Operator>> {
        match self.unspanned {
            UnspannedToken::Operator(operator) => Some(operator.spanned(self.span)),
            _ => None,
        }
    }

    pub fn extract_string(&self) -> Option<(Span, Span)> {
        match self.unspanned {
            UnspannedToken::String(span) => Some((span, self.span)),
            _ => None,
        }
    }

    pub fn extract_variable(&self) -> Option<(Span, Span)> {
        match self.unspanned {
            UnspannedToken::Variable(span) => Some((span, self.span)),
            _ => None,
        }
    }

    pub fn extract_external_command(&self) -> Option<(Span, Span)> {
        match self.unspanned {
            UnspannedToken::ExternalCommand(span) => Some((span, self.span)),
            _ => None,
        }
    }

    pub fn extract_external_word(&self) -> Option<Span> {
        match self.unspanned {
            UnspannedToken::ExternalWord => Some(self.span),
            _ => None,
        }
    }

    pub fn extract_glob_pattern(&self) -> Option<Span> {
        match self.unspanned {
            UnspannedToken::GlobPattern => Some(self.span),
            _ => None,
        }
    }

    pub fn extract_bare(&self) -> Option<Span> {
        match self.unspanned {
            UnspannedToken::Bare => Some(self.span),
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
        write!(f, "{}", self.node.span.slice(self.source))
    }
}
