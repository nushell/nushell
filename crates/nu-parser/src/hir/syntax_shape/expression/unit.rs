use crate::hir::syntax_shape::flat_shape::FlatShape;
use crate::hir::syntax_shape::ExpandSyntax;
use crate::hir::TokensIterator;
use crate::hir::{Expression, SpannedExpression};
use crate::parse::number::RawNumber;
use crate::parse::token_tree::BareType;
use crate::parse::unit::Unit;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::digit1;
use nom::combinator::{all_consuming, opt, value};
use nom::IResult;
use nu_errors::ParseError;
use nu_source::{
    b, DebugDocBuilder, HasSpan, PrettyDebugWithSource, Span, Spanned, SpannedItem, Text,
};

#[derive(Debug, Clone)]
pub struct UnitSyntax {
    pub unit: (RawNumber, Spanned<Unit>),
    pub span: Span,
}

impl UnitSyntax {
    pub fn into_expr(self, source: &Text) -> SpannedExpression {
        let UnitSyntax {
            unit: (number, unit),
            span,
        } = self;

        Expression::size(number.to_number(source), *unit).into_expr(span)
    }
}

impl PrettyDebugWithSource for UnitSyntax {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::typed(
            "unit",
            self.unit.0.pretty_debug(source) + b::space() + self.unit.1.pretty_debug(source),
        )
    }
}

impl HasSpan for UnitSyntax {
    fn span(&self) -> Span {
        self.span
    }
}

#[derive(Debug, Copy, Clone)]
pub struct UnitExpressionShape;

impl ExpandSyntax for UnitExpressionShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "unit expression"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<SpannedExpression, ParseError> {
        token_nodes
            .expand_syntax(UnitShape)
            .map(|unit| unit.into_expr(&token_nodes.source()))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct UnitShape;

impl ExpandSyntax for UnitShape {
    type Output = Result<UnitSyntax, ParseError>;

    fn name(&self) -> &'static str {
        "unit"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<UnitSyntax, ParseError> {
        let source = token_nodes.source();

        token_nodes.expand_token(BareType, |span| {
            let unit = unit_size(span.slice(&source), span);

            let (_, (number, unit)) = match unit {
                Err(_) => return Err(ParseError::mismatch("unit", "word".spanned(span))),
                Ok((number, unit)) => (number, unit),
            };

            Ok((
                FlatShape::Size {
                    number: number.span(),
                    unit: unit.span,
                },
                UnitSyntax {
                    unit: (number, unit),
                    span,
                },
            ))
        })
    }
}

fn unit_size(input: &str, bare_span: Span) -> IResult<&str, (RawNumber, Spanned<Unit>)> {
    let (input, digits) = digit1(input)?;

    let (input, dot) = opt(tag("."))(input)?;

    let (input, number) = match dot {
        Some(dot) => {
            let (input, rest) = digit1(input)?;
            (
                input,
                RawNumber::decimal(Span::new(
                    bare_span.start(),
                    bare_span.start() + digits.len() + dot.len() + rest.len(),
                )),
            )
        }

        None => (
            input,
            RawNumber::int(Span::new(
                bare_span.start(),
                bare_span.start() + digits.len(),
            )),
        ),
    };

    let (input, unit) = all_consuming(alt((
        value(Unit::Byte, alt((tag("B"), tag("b")))),
        value(Unit::Kilobyte, alt((tag("KB"), tag("kb"), tag("Kb")))),
        value(Unit::Megabyte, alt((tag("MB"), tag("mb"), tag("Mb")))),
        value(Unit::Gigabyte, alt((tag("GB"), tag("gb"), tag("Gb")))),
        value(Unit::Terabyte, alt((tag("TB"), tag("tb"), tag("Tb")))),
        value(Unit::Petabyte, alt((tag("PB"), tag("pb"), tag("Pb")))),
        value(Unit::Second, tag("s")),
        value(Unit::Minute, tag("m")),
        value(Unit::Hour, tag("h")),
        value(Unit::Day, tag("d")),
        value(Unit::Week, tag("w")),
        value(Unit::Month, tag("M")),
        value(Unit::Year, tag("y")),
    )))(input)?;

    let start_span = number.span().end();

    Ok((
        input,
        (number, unit.spanned(Span::new(start_span, bare_span.end()))),
    ))
}
