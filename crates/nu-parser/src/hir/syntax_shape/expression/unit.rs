use crate::hir::syntax_shape::{ExpandContext, ExpandSyntax};
use crate::parse::tokens::RawNumber;
use crate::parse::tokens::Token;
use crate::parse::tokens::UnspannedToken;
use crate::parse::unit::Unit;
use crate::{hir::TokensIterator, TokenNode};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::digit1;
use nom::combinator::{all_consuming, opt, value};
use nom::IResult;
use nu_errors::ParseError;
use nu_source::{b, DebugDocBuilder, HasSpan, PrettyDebugWithSource, Span, Spanned, SpannedItem};

#[derive(Debug, Clone)]
pub struct UnitSyntax {
    pub unit: (RawNumber, Spanned<Unit>),
    pub span: Span,
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
pub struct UnitShape;

impl ExpandSyntax for UnitShape {
    type Output = UnitSyntax;

    fn name(&self) -> &'static str {
        "unit"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<UnitSyntax, ParseError> {
        let peeked = token_nodes.peek_any().not_eof("unit")?;

        let span = match peeked.node {
            TokenNode::Token(Token {
                unspanned: UnspannedToken::Bare,
                span,
            }) => *span,
            _ => return Err(peeked.type_error("unit")),
        };

        let unit = unit_size(span.slice(context.source), span);

        let (_, (number, unit)) = match unit {
            Err(_) => return Err(ParseError::mismatch("unit", "word".spanned(span))),
            Ok((number, unit)) => (number, unit),
        };

        peeked.commit();
        Ok(UnitSyntax {
            unit: (number, unit),
            span,
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
