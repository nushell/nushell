use crate::hir::syntax_shape::FlatShape;
use crate::parse::parser::Number;
use bigdecimal::BigDecimal;
use nu_source::{b, DebugDocBuilder, HasSpan, PrettyDebugWithSource, Span, Text};
use num_bigint::BigInt;
use std::str::FromStr;

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
    pub fn as_flat_shape(&self) -> FlatShape {
        match self {
            RawNumber::Int(_) => FlatShape::Int,
            RawNumber::Decimal(_) => FlatShape::Decimal,
        }
    }

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
