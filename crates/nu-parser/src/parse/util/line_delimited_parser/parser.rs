use crate::parse::number::RawNumber;
use crate::parse::parser::{is_boundary, to_list};
use crate::parse::token_tree::SpannedToken;
use crate::parse::token_tree_builder::TokenTreeBuilder;
use nu_source::{HasSpan, NomSpan, Span, Spanned, SpannedItem};

use nom::branch::alt;
use nom::bytes::complete::{escaped, tag};
use nom::character::complete::*;
use nom::combinator::*;
use nom::multi::*;
use nom::IResult;
use nom_tracable::tracable_parser;

#[tracable_parser]
pub fn parse_line_with_separator<'a, 'b>(
    separator: &'b str,
    input: NomSpan<'a>,
) -> IResult<NomSpan<'a>, Spanned<Vec<SpannedToken>>> {
    let start = input.offset;
    let mut nodes = vec![];
    let mut next_input = input;

    loop {
        let node_result = to_list(leaf(separator))(next_input);

        let (after_node_input, next_nodes) = match node_result {
            Err(_) => break,
            Ok((after_node_input, next_node)) => (after_node_input, next_node),
        };

        nodes.extend(next_nodes);

        match separated_by(separator)(after_node_input) {
            Err(_) => {
                next_input = after_node_input;
                break;
            }
            Ok((input, s)) => {
                nodes.push(s);
                next_input = input;
            }
        }
    }

    let end = next_input.offset;

    Ok((next_input, nodes.spanned(Span::new(start, end))))
}

#[tracable_parser]
pub fn fallback_number_without(c: char) -> impl Fn(NomSpan) -> IResult<NomSpan, SpannedToken> {
    move |input| {
        let (input, number) = fallback_raw_number_without(c)(input)?;

        Ok((
            input,
            TokenTreeBuilder::spanned_number(number, number.span()),
        ))
    }
}

#[tracable_parser]
pub fn fallback_raw_number_without(c: char) -> impl Fn(NomSpan) -> IResult<NomSpan, RawNumber> {
    move |input| {
        let _anchoral = input;
        let start = input.offset;
        let (input, _neg) = opt(tag("-"))(input)?;
        let (input, _head) = digit1(input)?;
        let after_int_head = input;

        match input.fragment.chars().next() {
            None => return Ok((input, RawNumber::int(Span::new(start, input.offset)))),
            Some('.') => (),
            other if is_boundary(other) || other == Some(c) => {
                return Ok((input, RawNumber::int(Span::new(start, input.offset))))
            }
            _ => {
                return Err(nom::Err::Error(nom::error::make_error(
                    input,
                    nom::error::ErrorKind::Tag,
                )))
            }
        }

        let dot: IResult<NomSpan, NomSpan, (NomSpan, nom::error::ErrorKind)> = tag(".")(input);

        let input = match dot {
            Ok((input, _dot)) => input,

            // it's just an integer
            Err(_) => return Ok((input, RawNumber::int(Span::new(start, input.offset)))),
        };

        let tail_digits_result: IResult<NomSpan, _> = digit1(input);

        let (input, _tail) = match tail_digits_result {
            Ok((input, tail)) => (input, tail),
            Err(_) => {
                return Ok((
                    after_int_head,
                    RawNumber::int((start, after_int_head.offset)),
                ))
            }
        };

        let end = input.offset;

        let next = input.fragment.chars().next();

        if is_boundary(next) || next == Some(c) {
            Ok((input, RawNumber::decimal(Span::new(start, end))))
        } else {
            Err(nom::Err::Error(nom::error::make_error(
                input,
                nom::error::ErrorKind::Tag,
            )))
        }
    }
}

#[tracable_parser]
pub fn leaf(c: &str) -> impl Fn(NomSpan) -> IResult<NomSpan, SpannedToken> + '_ {
    move |input| {
        let separator = c.chars().next().unwrap_or_else(|| ',');

        let (input, node) = alt((
            fallback_number_without(separator),
            string,
            fallback_string_without(c),
        ))(input)?;

        Ok((input, node))
    }
}

#[tracable_parser]
pub fn separated_by(c: &str) -> impl Fn(NomSpan) -> IResult<NomSpan, SpannedToken> + '_ {
    move |input| {
        let left = input.offset;
        let (input, _) = tag(c)(input)?;
        let right = input.offset;

        Ok((input, TokenTreeBuilder::spanned_sep(Span::new(left, right))))
    }
}

#[tracable_parser]
pub fn dq_string(input: NomSpan) -> IResult<NomSpan, SpannedToken> {
    let start = input.offset;
    let (input, _) = char('"')(input)?;
    let start1 = input.offset;
    let (input, _) = escaped(
        none_of(r#"\""#),
        '\\',
        nom::character::complete::one_of(r#"\"rnt"#),
    )(input)?;

    let end1 = input.offset;
    let (input, _) = char('"')(input)?;
    let end = input.offset;
    Ok((
        input,
        TokenTreeBuilder::spanned_string(Span::new(start1, end1), Span::new(start, end)),
    ))
}

#[tracable_parser]
pub fn sq_string(input: NomSpan) -> IResult<NomSpan, SpannedToken> {
    let start = input.offset;
    let (input, _) = char('\'')(input)?;
    let start1 = input.offset;
    let (input, _) = many0(none_of("\'"))(input)?;
    let end1 = input.offset;
    let (input, _) = char('\'')(input)?;
    let end = input.offset;

    Ok((
        input,
        TokenTreeBuilder::spanned_string(Span::new(start1, end1), Span::new(start, end)),
    ))
}

#[tracable_parser]
pub fn string(input: NomSpan) -> IResult<NomSpan, SpannedToken> {
    alt((sq_string, dq_string))(input)
}

#[tracable_parser]
pub fn fallback_string_without(c: &str) -> impl Fn(NomSpan) -> IResult<NomSpan, SpannedToken> + '_ {
    move |input| {
        let start = input.offset;
        let (input, _) = many0(none_of(c))(input)?;
        let end = input.offset;

        Ok((
            input,
            TokenTreeBuilder::spanned_string(Span::new(start, end), Span::new(start, end)),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::token_tree_builder::TokenTreeBuilder::{self, self as b};
    use crate::parse::util::parse_line_with_separator;
    use crate::test_support::apply;
    use nom::IResult;

    use crate::parse::pipeline::PipelineElement;
    use crate::parse::token_tree::SpannedToken;
    use nu_source::NomSpan;
    use nu_source::PrettyDebugWithSource;

    use pretty_assertions::assert_eq;

    pub fn nodes(input: NomSpan) -> IResult<NomSpan, SpannedToken> {
        let (input, tokens) = parse_line_with_separator(",", input)?;
        let span = tokens.span;

        Ok((
            input,
            TokenTreeBuilder::spanned_pipeline(vec![PipelineElement::new(None, tokens)], span),
        ))
    }

    #[test]
    fn separators() {
        equal_tokens! {
            <nodes>
            r#""name","lastname","age""# ->  b::token_list(vec![
                b::string("name"),
                b::sep(","),
                b::string("lastname"),
                b::sep(","),
                b::string("age")
            ])
        }

        equal_tokens! {
            <nodes>
            r#""Andrés","Robalino",12"# ->  b::token_list(vec![
                b::string("Andrés"),
                b::sep(","),
                b::string("Robalino"),
                b::sep(","),
                b::int(12)
            ])
        }
    }

    #[test]
    fn strings() {
        equal_tokens! {
            <nodes>
            r#""andres""# ->  b::token_list(vec![b::string("andres")])
        }
    }

    #[test]
    fn numbers() {
        equal_tokens! {
            <nodes>
            "123" -> b::token_list(vec![b::int(123)])
        }

        equal_tokens! {
            <nodes>
            "-123" -> b::token_list(vec![b::int(-123)])
        }
    }
}
