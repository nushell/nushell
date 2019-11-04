use crate::data::meta::Span;
use crate::parser::hir::syntax_shape::{ExpandContext, ExpandSyntax, ParseError};
use crate::parser::parse::tokens::RawNumber;
use crate::parser::parse::unit::Unit;
use crate::parser::{hir::TokensIterator, RawToken, TokenNode};
use crate::prelude::*;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::digit1;
use nom::combinator::{all_consuming, opt, value};
use nom::IResult;
use std::fmt;

#[derive(Debug, Copy, Clone)]
pub struct UnitShape;

impl FormatDebug for Spanned<(Spanned<RawNumber>, Spanned<Unit>)> {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        let dict = indexmap::indexmap! {
            "number" => format!("{}", self.item.0.item.debug(source)),
            "unit" => format!("{}", self.item.1.debug(source)),
        };

        f.say_dict("unit", dict)
    }
}

impl ExpandSyntax for UnitShape {
    type Output = Spanned<(Spanned<RawNumber>, Spanned<Unit>)>;

    fn name(&self) -> &'static str {
        "unit"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Spanned<(Spanned<RawNumber>, Spanned<Unit>)>, ParseError> {
        let peeked = token_nodes.peek_any().not_eof("unit")?;

        let span = match peeked.node {
            TokenNode::Token(Spanned {
                item: RawToken::Bare,
                span,
            }) => span,
            _ => return Err(peeked.type_error("unit")),
        };

        let unit = unit_size(span.slice(context.source), *span);

        let (_, (number, unit)) = match unit {
            Err(_) => return Err(ParseError::mismatch("unit", "word".spanned(*span))),
            Ok((number, unit)) => (number, unit),
        };

        peeked.commit();
        Ok((number, unit).spanned(*span))
    }
}

fn unit_size(input: &str, bare_span: Span) -> IResult<&str, (Spanned<RawNumber>, Spanned<Unit>)> {
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

    let start_span = number.span.end();

    Ok((
        input,
        (number, unit.spanned(Span::new(start_span, bare_span.end()))),
    ))
}
