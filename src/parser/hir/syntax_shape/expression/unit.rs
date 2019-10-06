use crate::data::meta::Span;
use crate::parser::hir::syntax_shape::{ExpandContext, ExpandSyntax};
use crate::parser::parse::tokens::RawNumber;
use crate::parser::parse::unit::Unit;
use crate::parser::{hir::TokensIterator, RawToken, TokenNode};
use crate::prelude::*;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::digit1;
use nom::combinator::{all_consuming, opt, value};
use nom::IResult;

#[derive(Debug, Copy, Clone)]
pub struct UnitShape;

impl ExpandSyntax for UnitShape {
    type Output = Tagged<(Tagged<RawNumber>, Tagged<Unit>)>;

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Tagged<(Tagged<RawNumber>, Tagged<Unit>)>, ShellError> {
        let peeked = token_nodes.peek_any().not_eof("unit")?;

        let tag = match peeked.node {
            TokenNode::Token(Tagged {
                item: RawToken::Bare,
                tag,
            }) => tag,
            _ => return Err(peeked.type_error("unit")),
        };

        let unit = unit_size(tag.slice(context.source), *tag);

        let (_, (number, unit)) = match unit {
            Err(_) => {
                return Err(ShellError::type_error(
                    "unit",
                    "word".tagged(Tag::unknown()),
                ))
            }
            Ok((number, unit)) => (number, unit),
        };

        peeked.commit();
        Ok((number, unit).tagged(tag))
    }
}

fn unit_size(input: &str, bare_tag: Tag) -> IResult<&str, (Tagged<RawNumber>, Tagged<Unit>)> {
    let (input, digits) = digit1(input)?;

    let (input, dot) = opt(tag("."))(input)?;

    let (input, number) = match dot {
        Some(dot) => {
            let (input, rest) = digit1(input)?;
            (
                input,
                RawNumber::decimal((
                    bare_tag.span.start(),
                    bare_tag.span.start() + digits.len() + dot.len() + rest.len(),
                    bare_tag.anchor,
                )),
            )
        }

        None => (
            input,
            RawNumber::int((
                bare_tag.span.start(),
                bare_tag.span.start() + digits.len(),
                bare_tag.anchor,
            )),
        ),
    };

    let (input, unit) = all_consuming(alt((
        value(Unit::B, alt((tag("B"), tag("b")))),
        value(Unit::KB, alt((tag("KB"), tag("kb"), tag("Kb")))),
        value(Unit::MB, alt((tag("MB"), tag("mb"), tag("Mb")))),
        value(Unit::MB, alt((tag("GB"), tag("gb"), tag("Gb")))),
        value(Unit::MB, alt((tag("TB"), tag("tb"), tag("Tb")))),
        value(Unit::MB, alt((tag("PB"), tag("pb"), tag("Pb")))),
    )))(input)?;

    let start_span = number.tag.span.end();

    let unit_tag = Tag::new(
        bare_tag.anchor,
        Span::from((start_span, bare_tag.span.end())),
    );

    Ok((input, (number, unit.tagged(unit_tag))))
}
