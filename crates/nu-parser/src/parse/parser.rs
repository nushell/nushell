#![allow(unused)]

use crate::parse::{
    call_node::*, flag::*, operator::*, pipeline::*, token_tree::*, token_tree_builder::*,
    tokens::*, unit::*,
};
use nom;
use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::multi::*;
use nom::sequence::*;

use bigdecimal::BigDecimal;
use derive_new::new;
use enumflags2::BitFlags;
use log::trace;
use nom::dbg;
use nom::*;
use nom::{AsBytes, FindSubstring, IResult, InputLength, InputTake, Slice};
use nom_locate::{position, LocatedSpanEx};
use nom_tracable::{tracable_parser, HasTracableInfo, TracableInfo};
use nu_protocol::{Primitive, UntaggedValue};
use nu_source::{
    b, nom_input, DebugDocBuilder, HasSpan, NomSpan, PrettyDebug, PrettyDebugWithSource, Span,
    Spanned, SpannedItem, Tag,
};
use num_bigint::BigInt;
use num_traits::identities::Zero;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::str::FromStr;

macro_rules! cmp_operator {
    ($name:tt : $token:tt ) => {
        #[tracable_parser]
        pub fn $name(input: NomSpan) -> IResult<NomSpan, TokenNode> {
            let start = input.offset;
            let (input, tag) = tag($token)(input)?;
            let end = input.offset;

            Ok((
                input,
                TokenTreeBuilder::spanned_cmp_op(tag.fragment, Span::new(start, end)),
            ))
        }
    };
}

macro_rules! eval_operator {
    ($name:tt : $token:tt ) => {
        #[tracable_parser]
        pub fn $name(input: NomSpan) -> IResult<NomSpan, TokenNode> {
            let start = input.offset;
            let (input, tag) = tag($token)(input)?;
            let end = input.offset;

            Ok((
                input,
                TokenTreeBuilder::spanned_eval_op(tag.fragment, Span::new(start, end)),
            ))
        }
    };
}

cmp_operator! { gt:  ">"  }
cmp_operator! { lt:  "<"  }
cmp_operator! { gte: ">=" }
cmp_operator! { lte: "<=" }
cmp_operator! { eq:  "==" }
cmp_operator! { neq: "!=" }
cmp_operator! { cont: "=~" }
cmp_operator! { ncont: "!~" }
eval_operator! { dot: "." }
eval_operator! { dotdot: ".." }

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Number {
    Int(BigInt),
    Decimal(BigDecimal),
}

impl Into<Number> for &Number {
    fn into(self) -> Number {
        self.clone()
    }
}

impl Into<UntaggedValue> for Number {
    fn into(self) -> UntaggedValue {
        match self {
            Number::Int(i) => int(i),
            Number::Decimal(d) => decimal(d),
        }
    }
}

pub fn int(i: impl Into<BigInt>) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::Int(i.into()))
}

pub fn decimal(i: impl Into<BigDecimal>) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::Decimal(i.into()))
}

impl Into<UntaggedValue> for &Number {
    fn into(self) -> UntaggedValue {
        match self {
            Number::Int(i) => int(i.clone()),
            Number::Decimal(d) => decimal(d.clone()),
        }
    }
}

impl PrettyDebug for Number {
    fn pretty(&self) -> DebugDocBuilder {
        match self {
            Number::Int(int) => b::primitive(int),
            Number::Decimal(decimal) => b::primitive(decimal),
        }
    }
}

macro_rules! primitive_int {
    ($($ty:ty)*) => {
        $(
            impl From<$ty> for Number {
                fn from(int: $ty) -> Number {
                    Number::Int(BigInt::zero() + int)
                }
            }

            impl From<&$ty> for Number {
                fn from(int: &$ty) -> Number {
                    Number::Int(BigInt::zero() + *int)
                }
            }
        )*
    }
}

primitive_int!(i8 u8 i16 u16 i32 u32 i64 u64 i128 u128);

macro_rules! primitive_decimal {
    ($($ty:tt -> $from:tt),*) => {
        $(
            impl From<$ty> for Number {
                fn from(decimal: $ty) -> Number {
                    if let Some(num) = BigDecimal::$from(decimal) {
                        Number::Decimal(num)
                    } else {
                        unreachable!("Internal error: BigDecimal 'from' failed")
                    }
                }
            }

            impl From<&$ty> for Number {
                fn from(decimal: &$ty) -> Number {
                    if let Some(num) = BigDecimal::$from(*decimal) {
                        Number::Decimal(num)
                    } else {
                        unreachable!("Internal error: BigDecimal 'from' failed")
                    }
                }
            }
        )*
    }
}

primitive_decimal!(f32 -> from_f32, f64 -> from_f64);

impl std::ops::Mul for Number {
    type Output = Number;

    fn mul(self, other: Number) -> Number {
        match (self, other) {
            (Number::Int(a), Number::Int(b)) => Number::Int(a * b),
            (Number::Int(a), Number::Decimal(b)) => Number::Decimal(BigDecimal::from(a) * b),
            (Number::Decimal(a), Number::Int(b)) => Number::Decimal(a * BigDecimal::from(b)),
            (Number::Decimal(a), Number::Decimal(b)) => Number::Decimal(a * b),
        }
    }
}

// For literals
impl std::ops::Mul<u32> for Number {
    type Output = Number;

    fn mul(self, other: u32) -> Number {
        match self {
            Number::Int(left) => Number::Int(left * (other as i64)),
            Number::Decimal(left) => Number::Decimal(left * BigDecimal::from(other)),
        }
    }
}

impl Into<Number> for BigDecimal {
    fn into(self) -> Number {
        Number::Decimal(self)
    }
}

impl Into<Number> for BigInt {
    fn into(self) -> Number {
        Number::Int(self)
    }
}

#[tracable_parser]
pub fn number(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let (input, number) = raw_number(input)?;

    Ok((
        input,
        TokenTreeBuilder::spanned_number(number, number.span()),
    ))
}

#[tracable_parser]
pub fn raw_number(input: NomSpan) -> IResult<NomSpan, RawNumber> {
    let anchoral = input;
    let start = input.offset;
    let (input, neg) = opt(tag("-"))(input)?;
    let (input, head) = digit1(input)?;

    match input.fragment.chars().next() {
        None => return Ok((input, RawNumber::int(Span::new(start, input.offset)))),
        Some('.') => (),
        other if is_boundary(other) => {
            return Ok((input, RawNumber::int(Span::new(start, input.offset))))
        }
        _ => {
            return Err(nom::Err::Error(nom::error::make_error(
                input,
                nom::error::ErrorKind::Tag,
            )))
        }
    }

    let dotdot_result = dotdot(input);

    if let Ok((dotdot_input, _)) = dotdot_result {
        // If we see a `..` immediately after an integer, it's a range, not a decimal
        return Ok((input, RawNumber::int(Span::new(start, input.offset))));
    }

    let dot: IResult<NomSpan, NomSpan, (NomSpan, nom::error::ErrorKind)> = tag(".")(input);

    let input = match dot {
        Ok((input, dot)) => input,

        // it's just an integer
        Err(_) => return Ok((input, RawNumber::int(Span::new(start, input.offset)))),
    };

    let (input, tail) = digit1(input)?;

    let end = input.offset;

    let next = input.fragment.chars().next();

    if is_boundary(next) {
        Ok((input, RawNumber::decimal(Span::new(start, end))))
    } else {
        Err(nom::Err::Error(nom::error::make_error(
            input,
            nom::error::ErrorKind::Tag,
        )))
    }
}

#[tracable_parser]
pub fn operator(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let (input, operator) = alt((gte, lte, neq, gt, lt, eq, cont, ncont))(input)?;

    Ok((input, operator))
}

#[tracable_parser]
pub fn dq_string(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let start = input.offset;
    let (input, _) = char('"')(input)?;
    let start1 = input.offset;
    let (input, _) = many0(none_of("\""))(input)?;
    let end1 = input.offset;
    let (input, _) = char('"')(input)?;
    let end = input.offset;
    Ok((
        input,
        TokenTreeBuilder::spanned_string(Span::new(start1, end1), Span::new(start, end)),
    ))
}

#[tracable_parser]
pub fn sq_string(input: NomSpan) -> IResult<NomSpan, TokenNode> {
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
pub fn string(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    alt((sq_string, dq_string))(input)
}

#[tracable_parser]
pub fn external(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let start = input.offset;
    let (input, _) = tag("^")(input)?;
    let (input, bare) = take_while(is_file_char)(input)?;
    let end = input.offset;

    Ok((
        input,
        TokenTreeBuilder::spanned_external_command(bare, Span::new(start, end)),
    ))
}

fn word<'a, T, U, V>(
    start_predicate: impl Fn(NomSpan<'a>) -> IResult<NomSpan<'a>, U>,
    next_predicate: impl Fn(NomSpan<'a>) -> IResult<NomSpan<'a>, V> + Copy,
    into: impl Fn(Span) -> T,
) -> impl Fn(NomSpan<'a>) -> IResult<NomSpan<'a>, T> {
    move |input: NomSpan| {
        let start = input.offset;

        let (input, _) = start_predicate(input)?;
        let (input, _) = many0(next_predicate)(input)?;

        let next_char = &input.fragment.chars().nth(0);

        match next_char {
            Some('.') => {}
            Some(next_char)
                if is_external_word_char(*next_char) || is_glob_specific_char(*next_char) =>
            {
                return Err(nom::Err::Error(nom::error::make_error(
                    input,
                    nom::error::ErrorKind::TakeWhile1,
                )));
            }
            _ => {}
        }

        let end = input.offset;

        Ok((input, into(Span::new(start, end))))
    }
}

pub fn matches(cond: fn(char) -> bool) -> impl Fn(NomSpan) -> IResult<NomSpan, NomSpan> + Copy {
    move |input: NomSpan| match input.iter_elements().next() {
        Option::Some(c) if cond(c) => {
            let len_utf8 = c.len_utf8();
            Ok((input.slice(len_utf8..), input.slice(0..len_utf8)))
        }
        _ => Err(nom::Err::Error(nom::error::ParseError::from_error_kind(
            input,
            nom::error::ErrorKind::Many0,
        ))),
    }
}

#[tracable_parser]
pub fn pattern(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    word(
        start_pattern,
        matches(is_glob_char),
        TokenTreeBuilder::spanned_pattern,
    )(input)
}

#[tracable_parser]
pub fn start_pattern(input: NomSpan) -> IResult<NomSpan, NomSpan> {
    alt((take_while1(is_dot), matches(is_start_glob_char)))(input)
}

#[tracable_parser]
pub fn filename(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let start_pos = input.offset;

    let (mut input, mut saw_special) = match start_file_char(input) {
        Err(err) => return Err(err),
        Ok((input, special)) => (input, special),
    };

    loop {
        if saw_special.is_empty() {
            match continue_file_char(input) {
                Err(_) => {
                    return Ok((
                        input,
                        TokenTreeBuilder::spanned_bare((start_pos, input.offset)),
                    ))
                }
                Ok((next_input, special)) => {
                    saw_special |= special;
                    input = next_input;
                }
            }
        } else {
            let rest = after_sep_file(input);

            let (input, span, updated_special) = match rest {
                Err(_) => (input, (start_pos, input.offset), saw_special),
                Ok((input, new_special)) => {
                    (input, (start_pos, input.offset), saw_special | new_special)
                }
            };

            if updated_special.contains(SawSpecial::Glob) {
                return Ok((input, TokenTreeBuilder::spanned_pattern(span)));
            } else {
                return Ok((input, TokenTreeBuilder::spanned_bare(span)));
            }
        }
    }
}

#[derive(BitFlags, Copy, Clone, Eq, PartialEq)]
enum SawSpecial {
    PathSeparator = 0b01,
    Glob = 0b10,
}

#[tracable_parser]
fn start_file_char(input: NomSpan) -> IResult<NomSpan, BitFlags<SawSpecial>> {
    let path_sep_result = special_file_char(input);

    if let Ok((input, special)) = path_sep_result {
        return Ok((input, special));
    }

    start_filename(input).map(|(input, output)| (input, BitFlags::empty()))
}

#[tracable_parser]
fn continue_file_char(input: NomSpan) -> IResult<NomSpan, BitFlags<SawSpecial>> {
    let path_sep_result = special_file_char(input);

    if let Ok((input, special)) = path_sep_result {
        return Ok((input, special));
    }

    matches(is_file_char)(input).map(|(input, _)| (input, BitFlags::empty()))
}

#[tracable_parser]
fn special_file_char(input: NomSpan) -> IResult<NomSpan, BitFlags<SawSpecial>> {
    if let Ok((input, _)) = matches(is_path_separator)(input) {
        return Ok((input, BitFlags::empty() | SawSpecial::PathSeparator));
    }

    let (input, _) = matches(is_glob_specific_char)(input)?;

    Ok((input, BitFlags::empty() | SawSpecial::Glob))
}

#[tracable_parser]
fn after_sep_file(input: NomSpan) -> IResult<NomSpan, BitFlags<SawSpecial>> {
    fn after_sep_char(c: char) -> bool {
        is_external_word_char(c) || is_file_char(c) || c == '.'
    }

    let start = input.offset;
    let original_input = input;
    let mut input = input;

    let (input, after_glob) = take_while1(after_sep_char)(input)?;

    let slice = original_input.slice(0..input.offset - start);

    let saw_special = if slice.fragment.chars().any(is_glob_specific_char) {
        BitFlags::empty() | SawSpecial::Glob
    } else {
        BitFlags::empty()
    };

    Ok((input, saw_special))
}

pub fn start_filename(input: NomSpan) -> IResult<NomSpan, NomSpan> {
    alt((take_while1(is_dot), matches(is_start_file_char)))(input)
}

#[tracable_parser]
pub fn member(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    word(
        matches(is_start_member_char),
        matches(is_member_char),
        TokenTreeBuilder::spanned_bare,
    )(input)
}

#[tracable_parser]
pub fn ident(input: NomSpan) -> IResult<NomSpan, Tag> {
    word(matches(is_id_start), matches(is_id_continue), Tag::from)(input)
}

#[tracable_parser]
pub fn external_word(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let start = input.offset;
    let (input, _) = take_while1(is_external_word_char)(input)?;
    let end = input.offset;

    Ok((input, TokenTreeBuilder::spanned_external_word((start, end))))
}

#[tracable_parser]
pub fn var(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let start = input.offset;
    let (input, _) = tag("$")(input)?;
    let (input, bare) = ident(input)?;
    let end = input.offset;

    Ok((
        input,
        TokenTreeBuilder::spanned_var(bare, Span::new(start, end)),
    ))
}

fn tight<'a>(
    parser: impl Fn(NomSpan<'a>) -> IResult<NomSpan<'a>, Vec<TokenNode>>,
) -> impl Fn(NomSpan<'a>) -> IResult<NomSpan<'a>, Vec<TokenNode>> {
    move |input: NomSpan| {
        let mut result = vec![];
        let (input, head) = parser(input)?;
        result.extend(head);

        let (input, tail) = opt(alt((many1(range_continuation), many1(dot_member))))(input)?;

        let next_char = &input.fragment.chars().nth(0);

        if is_boundary(*next_char) {
            if let Some(tail) = tail {
                for tokens in tail {
                    result.extend(tokens);
                }
            }

            Ok((input, result))
        } else {
            Err(nom::Err::Error(nom::error::make_error(
                input,
                nom::error::ErrorKind::Many0,
            )))
        }
    }
}

#[tracable_parser]
pub fn flag(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let start = input.offset;
    let (input, _) = tag("--")(input)?;
    let (input, bare) = filename(input)?;
    let end = input.offset;

    Ok((
        input,
        TokenTreeBuilder::spanned_flag(bare.span(), Span::new(start, end)),
    ))
}

#[tracable_parser]
pub fn shorthand(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let start = input.offset;
    let (input, _) = tag("-")(input)?;
    let (input, bare) = filename(input)?;
    let end = input.offset;

    Ok((
        input,
        TokenTreeBuilder::spanned_shorthand(bare.span(), Span::new(start, end)),
    ))
}

#[tracable_parser]
pub fn leaf(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let (input, node) = alt((number, string, operator, flag, shorthand, var, external))(input)?;

    Ok((input, node))
}

#[tracable_parser]
pub fn token_list(input: NomSpan) -> IResult<NomSpan, Spanned<Vec<TokenNode>>> {
    let start = input.offset;
    let mut node_list = vec![];

    let mut next_input = input;
    let mut before_space_input: Option<NomSpan> = None;
    let mut final_space_tokens = 0;

    loop {
        let node_result = tight_node(next_input);

        let (after_node_input, next_nodes) = match node_result {
            Err(_) => {
                if let Some(before_space_input) = before_space_input {
                    next_input = before_space_input;

                    for _ in 0..final_space_tokens {
                        node_list.pop();
                    }
                }

                break;
            }
            Ok((after_node_input, next_node)) => (after_node_input, next_node),
        };

        node_list.extend(next_nodes);

        // Special case that allows a parenthesized expression to immediate follow another
        // token without a space, which could represent a type annotation.
        let maybe_type = delimited_paren(after_node_input);

        let after_maybe_type_input = match maybe_type {
            Err(_) => after_node_input,
            Ok((after_maybe_type_input, parens)) => {
                node_list.push(parens);
                after_maybe_type_input
            }
        };

        let maybe_space = any_space(after_maybe_type_input);

        let after_space_input = match maybe_space {
            Err(_) => {
                next_input = after_maybe_type_input;

                break;
            }
            Ok((after_space_input, space)) => {
                final_space_tokens = space.len();
                node_list.extend(space);
                before_space_input = Some(after_maybe_type_input);
                after_space_input
            }
        };

        next_input = after_space_input;
    }

    let end = next_input.offset;

    Ok((next_input, node_list.spanned(Span::new(start, end))))
}

#[tracable_parser]
pub fn spaced_token_list(input: NomSpan) -> IResult<NomSpan, Spanned<Vec<TokenNode>>> {
    let start = input.offset;
    let (input, pre_ws) = opt(any_space)(input)?;
    let (input, items) = token_list(input)?;
    let (input, post_ws) = opt(any_space)(input)?;
    let end = input.offset;

    let mut out = vec![];

    if let Some(pre_ws) = pre_ws {
        out.extend(pre_ws)
    }
    out.extend(items.item);
    if let Some(post_ws) = post_ws {
        out.extend(post_ws)
    }

    Ok((input, out.spanned(Span::new(start, end))))
}

fn make_token_list(
    first: Vec<TokenNode>,
    list: Vec<(Vec<TokenNode>, Vec<TokenNode>)>,
    sp_right: Option<TokenNode>,
) -> Vec<TokenNode> {
    let mut nodes = vec![];

    nodes.extend(first);

    for (sep, list) in list {
        nodes.extend(sep);
        nodes.extend(list);
    }

    if let Some(sp_right) = sp_right {
        nodes.push(sp_right);
    }

    nodes
}

#[tracable_parser]
pub fn separator(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let left = input.offset;
    let (input, ws1) = alt((tag(";"), tag("\n")))(input)?;
    let right = input.offset;

    Ok((input, TokenTreeBuilder::spanned_sep(Span::new(left, right))))
}

#[tracable_parser]
pub fn whitespace(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let left = input.offset;
    let (input, ws1) = space1(input)?;
    let right = input.offset;

    Ok((input, TokenTreeBuilder::spanned_ws(Span::new(left, right))))
}

#[tracable_parser]
pub fn any_space(input: NomSpan) -> IResult<NomSpan, Vec<TokenNode>> {
    let left = input.offset;
    let (input, tokens) = many1(alt((whitespace, separator, comment)))(input)?;
    let right = input.offset;

    Ok((input, tokens))
}

#[tracable_parser]
pub fn comment(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let left = input.offset;
    let (input, start) = tag("#")(input)?;
    let (input, rest) = not_line_ending(input)?;
    let right = input.offset;

    let span = (start.offset + 1, right);

    Ok((
        input,
        TokenTreeBuilder::spanned_comment(span, Span::new(left, right)),
    ))
}

pub fn delimited(
    input: NomSpan,
    delimiter: Delimiter,
) -> IResult<NomSpan, (Span, Span, Spanned<Vec<TokenNode>>)> {
    let left = input.offset;
    let (input, open_span) = tag(delimiter.open())(input)?;
    let (input, inner_items) = opt(spaced_token_list)(input)?;
    let (input, close_span) = tag(delimiter.close())(input)?;
    let right = input.offset;

    let mut items = vec![];

    if let Some(inner_items) = inner_items {
        items.extend(inner_items.item);
    }

    Ok((
        input,
        (
            Span::from(open_span),
            Span::from(close_span),
            items.spanned(Span::new(left, right)),
        ),
    ))
}

#[tracable_parser]
pub fn delimited_paren(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let (input, (left, right, tokens)) = delimited(input, Delimiter::Paren)?;

    Ok((
        input,
        TokenTreeBuilder::spanned_parens(tokens.item, (left, right), tokens.span),
    ))
}

#[tracable_parser]
pub fn delimited_square(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let (input, (left, right, tokens)) = delimited(input, Delimiter::Square)?;

    Ok((
        input,
        TokenTreeBuilder::spanned_square(tokens.item, (left, right), tokens.span),
    ))
}

#[tracable_parser]
pub fn delimited_brace(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let (input, (left, right, tokens)) = delimited(input, Delimiter::Brace)?;

    Ok((
        input,
        TokenTreeBuilder::spanned_square(tokens.item, (left, right), tokens.span),
    ))
}

#[tracable_parser]
pub fn raw_call(input: NomSpan) -> IResult<NomSpan, Spanned<CallNode>> {
    let left = input.offset;
    let (input, items) = token_list(input)?;
    let right = input.offset;

    Ok((
        input,
        TokenTreeBuilder::spanned_call(items.item, Span::new(left, right)),
    ))
}

#[tracable_parser]
pub fn range_continuation(input: NomSpan) -> IResult<NomSpan, Vec<TokenNode>> {
    let original = input;

    let mut result = vec![];

    let (input, dotdot_result) = dotdot(input)?;
    result.push(dotdot_result);
    let (input, node_result) = tight_node(input)?;
    result.extend(node_result);

    Ok((input, result))
}

#[tracable_parser]
pub fn dot_member(input: NomSpan) -> IResult<NomSpan, Vec<TokenNode>> {
    let (input, dot_result) = dot(input)?;
    let (input, member_result) = any_member(input)?;

    Ok((input, vec![dot_result, member_result]))
}

#[tracable_parser]
pub fn any_member(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    alt((number, string, member))(input)
}

#[tracable_parser]
pub fn tight_node(input: NomSpan) -> IResult<NomSpan, Vec<TokenNode>> {
    alt((
        tight(to_list(leaf)),
        tight(to_list(filename)),
        tight(to_list(pattern)),
        to_list(comment),
        to_list(external_word),
        tight(to_list(delimited_paren)),
        tight(to_list(delimited_brace)),
        tight(to_list(delimited_square)),
    ))(input)
}

fn to_list(
    parser: impl Fn(NomSpan) -> IResult<NomSpan, TokenNode>,
) -> impl Fn(NomSpan) -> IResult<NomSpan, Vec<TokenNode>> {
    move |input| {
        let (input, next) = parser(input)?;

        Ok((input, vec![next]))
    }
}

#[tracable_parser]
pub fn nodes(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let (input, tokens) = token_list(input)?;

    Ok((
        input,
        TokenTreeBuilder::spanned_token_list(tokens.item, tokens.span),
    ))
}

#[tracable_parser]
pub fn pipeline(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let start = input.offset;
    let (input, head) = spaced_token_list(input)?;
    let (input, items) = many0(tuple((tag("|"), spaced_token_list)))(input)?;

    if input.input_len() != 0 {
        return Err(Err::Error(error_position!(
            input,
            nom::error::ErrorKind::Eof
        )));
    }

    let end = input.offset;

    let head_span = head.span;
    let mut all_items: Vec<PipelineElement> = vec![PipelineElement::new(None, head)];

    all_items.extend(items.into_iter().map(|(pipe, items)| {
        let items_span = items.span;
        PipelineElement::new(Some(Span::from(pipe)), items)
    }));

    Ok((
        input,
        TokenTreeBuilder::spanned_pipeline(all_items, Span::new(start, end)),
    ))
}

#[tracable_parser]
pub fn module(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let (input, tokens) = spaced_token_list(input)?;

    if input.input_len() != 0 {
        return Err(Err::Error(error_position!(
            input,
            nom::error::ErrorKind::Eof
        )));
    }

    Ok((
        input,
        TokenTreeBuilder::spanned_token_list(tokens.item, tokens.span),
    ))
}

fn parse_int<T>(frag: &str, neg: Option<T>) -> i64 {
    if let Ok(int) = FromStr::from_str(frag) {
        match neg {
            None => int,
            Some(_) => -int,
        }
    } else {
        unreachable!("Internal error: parse_int failed");
    }
}

fn is_boundary(c: Option<char>) -> bool {
    match c {
        None => true,
        Some(')') | Some(']') | Some('}') | Some('(') => true,
        Some(c) if c.is_whitespace() => true,
        _ => false,
    }
}

fn is_external_word_char(c: char) -> bool {
    match c {
        ';' | '|' | '#' | '-' | '"' | '\'' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '`'
        | '.' => false,
        other if other.is_whitespace() => false,
        _ => true,
    }
}

/// These characters appear in globs and not bare words
fn is_glob_specific_char(c: char) -> bool {
    c == '*' || c == '?'
}

fn is_start_glob_char(c: char) -> bool {
    is_start_file_char(c) || is_glob_specific_char(c) || c == '.'
}

fn is_glob_char(c: char) -> bool {
    is_file_char(c) || is_glob_specific_char(c)
}

fn is_dot(c: char) -> bool {
    c == '.'
}

fn is_path_separator(c: char) -> bool {
    match c {
        '\\' | '/' | ':' => true,
        _ => false,
    }
}

fn is_start_file_char(c: char) -> bool {
    match c {
        '+' => false,
        _ if c.is_alphanumeric() => true,
        '\\' => true,
        '/' => true,
        '_' => true,
        '-' => true,
        '~' => true,
        '.' => true,
        _ => false,
    }
}

fn is_file_char(c: char) -> bool {
    match c {
        '+' => true,
        _ if c.is_alphanumeric() => true,
        '\\' => true,
        '/' => true,
        '_' => true,
        '-' => true,
        '=' => true,
        '~' => true,
        ':' => true,
        '?' => true,
        _ => false,
    }
}

fn is_start_member_char(c: char) -> bool {
    match c {
        _ if c.is_alphanumeric() => true,
        '_' => true,
        '-' => true,
        _ => false,
    }
}

fn is_member_char(c: char) -> bool {
    match c {
        _ if c.is_alphanumeric() => true,
        '_' => true,
        '-' => true,
        _ => false,
    }
}

fn is_id_start(c: char) -> bool {
    unicode_xid::UnicodeXID::is_xid_start(c)
}

fn is_id_continue(c: char) -> bool {
    unicode_xid::UnicodeXID::is_xid_continue(c)
        || match c {
            '-' => true,
            '?' => true,
            '!' => true,
            _ => false,
        }
}

fn is_member_start(c: char) -> bool {
    match c {
        '"' | '\'' => true,
        '1'..='9' => true,

        other if is_id_start(other) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::token_tree_builder::TokenTreeBuilder as b;
    use crate::parse::token_tree_builder::{CurriedToken, TokenTreeBuilder};
    use pretty_assertions::assert_eq;

    pub type CurriedNode<T> = Box<dyn FnOnce(&mut TokenTreeBuilder) -> T + 'static>;

    macro_rules! equal_tokens {
        ($source:tt -> $tokens:expr) => {
            let result = apply(pipeline, "pipeline", $source);
            let (expected_tree, expected_source) = TokenTreeBuilder::build($tokens);

            if result != expected_tree {
                let debug_result = format!("{}", result.debug($source));
                let debug_expected = format!("{}", expected_tree.debug(&expected_source));

                if debug_result == debug_expected {
                    assert_eq!(
                        result, expected_tree,
                        "NOTE: actual and expected had equivalent debug serializations, source={:?}, debug_expected={:?}",
                        $source,
                        debug_expected
                    )
                } else {
                    assert_eq!(debug_result, debug_expected)
                }
            }
        };

        (<$parser:tt> $source:tt -> $tokens:expr) => {
            let result = apply($parser, stringify!($parser), $source);

            let (expected_tree, expected_source) = TokenTreeBuilder::build($tokens);

            if result != expected_tree {
                let debug_result = format!("{}", result.debug($source));
                let debug_expected = format!("{}", expected_tree.debug(&expected_source));

                if debug_result == debug_expected {
                    assert_eq!(
                        result, expected_tree,
                        "NOTE: actual and expected had equivalent debug serializations, source={:?}, debug_expected={:?}",
                        $source,
                        debug_expected
                    )
                } else {
                    assert_eq!(debug_result, debug_expected)
                }
            }
        };

    }

    #[test]
    fn test_integer() {
        equal_tokens! {
            <nodes>
            "123" -> b::token_list(vec![b::int(123)])
        }

        equal_tokens! {
            <nodes>
            "-123" -> b::token_list(vec![b::int(-123)])
        }
    }

    #[test]
    fn test_gt_operator() {
        equal_tokens! {
            <nodes>
            ">" -> b::token_list(vec![b::op(">")])
        }
    }

    #[test]
    fn test_gte_operator() {
        equal_tokens! {
            <nodes>
            ">=" -> b::token_list(vec![b::op(">=")])
        }
    }

    #[test]
    fn test_lt_operator() {
        equal_tokens! {
            <nodes>
            "<" -> b::token_list(vec![b::op("<")])
        }
    }

    #[test]
    fn test_lte_operator() {
        equal_tokens! {
            <nodes>
            "<=" -> b::token_list(vec![b::op("<=")])
        }
    }

    #[test]
    fn test_eq_operator() {
        equal_tokens! {
            <nodes>
            "==" -> b::token_list(vec![b::op("==")])
        }
    }

    #[test]
    fn test_ne_operator() {
        equal_tokens! {
        <nodes>
        "!=" -> b::token_list(vec![b::op("!=")])
        }
    }

    #[test]
    fn test_sim_operator() {
        equal_tokens! {
            <nodes>
            "=~" -> b::token_list(vec![b::op("=~")])
        }
    }

    #[test]
    fn test_nsim_operator() {
        equal_tokens! {
            <nodes>
            "!~" -> b::token_list(vec![b::op("!~")])
        }
    }

    #[test]
    fn test_string() {
        equal_tokens! {
            <nodes>
            r#""hello world""# -> b::token_list(vec![b::string("hello world")])
        }

        equal_tokens! {
            <nodes>
            r#"'hello world'"# -> b::token_list(vec![b::string("hello world")])
        }
    }

    #[test]
    fn test_bare() {
        equal_tokens! {
            <nodes>
            "hello" -> b::token_list(vec![b::bare("hello")])
        }
    }

    #[test]
    fn test_unit_sizes() {
        equal_tokens! {
            <nodes>
            "450MB" -> b::token_list(vec![b::bare("450MB")])
        }
    }
    #[test]
    fn test_simple_path() {
        equal_tokens! {
            <nodes>
            "chrome.exe" -> b::token_list(vec![b::bare("chrome"), b::dot(), b::bare("exe")])
        }

        equal_tokens! {
            <nodes>
            ".azure" -> b::token_list(vec![b::bare(".azure")])
        }

        equal_tokens! {
            <nodes>
            r"C:\windows\system.dll" -> b::token_list(vec![b::bare(r"C:\windows\system.dll")])
        }

        equal_tokens! {
            <nodes>
            r"C:\Code\-testing\my_tests.js" -> b::token_list(vec![b::bare(r"C:\Code\-testing\my_tests.js")])
        }

        equal_tokens! {
            <nodes>
            r"C:\Users\example\AppData\Local\Temp\.tmpZ4TVQ2\cd_test_8" -> b::token_list(vec![b::bare(r"C:\Users\example\AppData\Local\Temp\.tmpZ4TVQ2\cd_test_8")])
        }

        equal_tokens! {
            <pipeline>
            r"cd C:\Users\wycat\AppData\Local\Temp\.tmpaj5JKi\cd_test_11" ->  b::pipeline(vec![vec![
                b::bare("cd"),
                b::sp(),
                b::bare(r"C:\Users\wycat\AppData\Local\Temp\.tmpaj5JKi\cd_test_11")
            ]])
        }
    }

    #[test]
    fn test_flag() {
        equal_tokens! {
            <nodes>
            "--amigos" -> b::token_list(vec![b::flag("arepas")])
        }

        equal_tokens! {
            <nodes>
            "--all-amigos" -> b::token_list(vec![b::flag("all-amigos")])
        }
    }

    #[test]
    fn test_shorthand_flag() {
        equal_tokens! {
            <nodes>
            "-katz" -> b::token_list(vec![b::shorthand("katz")])
        }
    }

    #[test]
    fn test_variable() {
        equal_tokens! {
            <nodes>
            "$it" -> b::token_list(vec![b::var("it")])
        }

        equal_tokens! {
            <nodes>
            "$name" -> b::token_list(vec![b::var("name")])
        }
    }

    #[test]
    fn test_external() {
        equal_tokens! {
            <nodes>
            "^ls" -> b::token_list(vec![b::external_command("ls")])
        }
    }

    #[test]
    fn test_dot_prefixed_name() {
        equal_tokens! {
            <nodes>
            ".azure" -> b::token_list(vec![b::bare(".azure")])
        }
    }

    #[test]
    fn test_delimited_paren() {
        equal_tokens! {
            <nodes>
            "(abc)" -> b::token_list(vec![b::parens(vec![b::bare("abc")])])
        }

        equal_tokens! {
            <nodes>
            "(  abc  )" -> b::token_list(vec![b::parens(vec![b::ws("  "), b::bare("abc"), b::ws("  ")])])
        }

        equal_tokens! {
            <nodes>
            "(  abc def )" -> b::token_list(vec![b::parens(vec![b::ws("  "), b::bare("abc"), b::sp(), b::bare("def"), b::sp()])])
        }

        equal_tokens! {
            <nodes>
            "(  abc def 123 456GB )" -> b::token_list(vec![b::parens(vec![
                b::ws("  "), b::bare("abc"), b::sp(), b::bare("def"), b::sp(), b::int(123), b::sp(), b::bare("456GB"), b::sp()
            ])])
        }
    }

    #[test]
    fn test_delimited_square() {
        equal_tokens! {
            <nodes>
            "[abc]" -> b::token_list(vec![b::square(vec![b::bare("abc")])])
        }

        equal_tokens! {
            <nodes>
            "[  abc  ]" -> b::token_list(vec![b::square(vec![b::ws("  "), b::bare("abc"), b::ws("  ")])])
        }

        equal_tokens! {
            <nodes>
            "[  abc def ]" -> b::token_list(vec![b::square(vec![b::ws("  "), b::bare("abc"), b::sp(), b::bare("def"), b::sp()])])
        }

        equal_tokens! {
            <nodes>
            "[  abc def 123 456GB ]" -> b::token_list(vec![b::square(vec![
                b::ws("  "), b::bare("abc"), b::sp(), b::bare("def"), b::sp(), b::int(123), b::sp(), b::bare("456GB"), b::sp()
            ])])
        }
    }

    #[test]
    fn test_range() {
        let _ = pretty_env_logger::try_init();

        equal_tokens! {
            <nodes>
            "0..2" -> b::token_list(vec![b::int(0), b::dotdot(), b::int(2)])
        }
    }

    #[test]
    fn test_path() {
        let _ = pretty_env_logger::try_init();

        equal_tokens! {
            <nodes>
            "$it.print" -> b::token_list(vec![b::var("it"), b::dot(), b::bare("print")])
        }

        equal_tokens! {
            <nodes>
            "$it.0" -> b::token_list(vec![b::var("it"), b::dot(), b::int(0)])
        }

        equal_tokens! {
            <nodes>
            "$head.part1.part2" -> b::token_list(vec![b::var("head"), b::dot(), b::bare("part1"), b::dot(), b::bare("part2")])
        }

        equal_tokens! {
            <nodes>
            "( hello ).world" -> b::token_list(vec![b::parens(vec![b::sp(), b::bare("hello"), b::sp()]), b::dot(), b::bare("world")])
        }

        equal_tokens! {
            <nodes>
            r#"( hello )."world""# -> b::token_list(vec![b::parens(vec![b::sp(), b::bare("hello"), b::sp()]), b::dot(), b::string("world")])
        }
    }

    #[test]
    fn test_nested_path() {
        equal_tokens! {
            <nodes>
            r#"( $it.is."great news".right yep $yep )."world""# -> b::token_list(
                vec![
                    b::parens(vec![
                        b::sp(),
                        b::var("it"),
                        b::dot(),
                        b::bare("is"),
                        b::dot(),
                        b::string("great news"),
                        b::dot(),
                        b::bare("right"),
                        b::sp(),
                        b::bare("yep"),
                        b::sp(),
                        b::var("yep"),
                        b::sp()
                    ]),
                    b::dot(), b::string("world")]
            )
        }

        equal_tokens! {
            <nodes>
            r#"$it."are PAS".0"# -> b::token_list(
                vec![
                    b::var("it"),
                    b::dot(),
                    b::string("are PAS"),
                    b::dot(),
                    b::int(0),
                    ]
            )
        }
    }

    #[test]
    fn test_smoke_single_command() {
        equal_tokens! {
            <nodes>
            "git add ." -> b::token_list(vec![b::bare("git"), b::sp(), b::bare("add"), b::sp(), b::bare(".")])
        }
    }

    #[test]
    fn test_smoke_single_command_open() {
        equal_tokens! {
            <nodes>
            "open Cargo.toml" -> b::token_list(vec![b::bare("open"), b::sp(), b::bare("Cargo"), b::dot(), b::bare("toml")])
        }
    }

    #[test]
    fn test_smoke_single_command_select() {
        equal_tokens! {
            <nodes>
            "select package.version" -> b::token_list(vec![b::bare("select"), b::sp(), b::bare("package"), b::dot(), b::bare("version")])
        }
    }

    #[test]
    fn test_smoke_single_command_it() {
        equal_tokens! {
            <nodes>
            "echo $it" -> b::token_list(vec![b::bare("echo"), b::sp(), b::var("it")])
        }
    }

    #[test]
    fn test_smoke_single_command_open_raw() {
        equal_tokens! {
            <nodes>
            "open Cargo.toml --raw" -> b::token_list(vec![b::bare("open"), b::sp(), b::bare("Cargo"), b::dot(), b::bare("toml"), b::sp(), b::flag("raw")])
        }
    }

    #[test]
    fn test_smoke_single_command_open_r() {
        equal_tokens! {
            <nodes>
            "open Cargo.toml -r" -> b::token_list(vec![b::bare("open"), b::sp(), b::bare("Cargo"), b::dot(), b::bare("toml"), b::sp(), b::shorthand("r")])
        }
    }

    #[test]
    fn test_smoke_single_command_config() {
        equal_tokens! {
            <nodes>
            "config --set tabs 2" -> b::token_list(vec![b::bare("config"), b::sp(), b::flag("set"), b::sp(), b::bare("tabs"), b::sp(), b::int(2)])
        }
    }

    #[test]
    fn test_smoke_single_command_inc() {
        equal_tokens! {
            <nodes>
            "inc --patch package.version" -> b::token_list(
                vec![
                    b::bare("inc"),
                    b::sp(),
                    b::flag("patch"),
                    b::sp(),
                    b::bare("package"), b::dot(), b::bare("version")
                ]
            )
        }
    }

    #[test]
    fn test_external_word() {
        let _ = pretty_env_logger::try_init();

        equal_tokens!(
            "cargo +nightly run" ->
            b::pipeline(vec![vec![
                b::bare("cargo"),
                b::sp(),
                b::external_word("+nightly"),
                b::sp(),
                b::bare("run")
            ]])
        );

        equal_tokens!(
            "rm foo%bar" ->
            b::pipeline(vec![vec![
                b::bare("rm"), b::sp(), b::external_word("foo%bar")
            ]])
        );

        equal_tokens!(
            "rm foo%bar" ->
            b::pipeline(vec![vec![
                b::bare("rm"), b::sp(), b::external_word("foo%bar"),
            ]])
        );
    }

    #[test]
    fn test_pipeline() {
        let _ = pretty_env_logger::try_init();

        equal_tokens! {
            "sys | echo" -> b::pipeline(vec![
                vec![
                    b::bare("sys"), b::sp()
                ],
                vec![
                    b::sp(), b::bare("echo")
                ]
            ])
        }
    }

    #[test]
    fn test_patterns() {
        equal_tokens! {
            <pipeline>
            "cp ../formats/*" -> b::pipeline(vec![vec![b::bare("cp"), b::sp(), b::pattern("../formats/*")]])
        }

        equal_tokens! {
            <pipeline>
            "cp * /dev/null" -> b::pipeline(vec![vec![b::bare("cp"), b::sp(), b::pattern("*"), b::sp(), b::bare("/dev/null")]])
        }
    }

    #[test]
    fn test_pseudo_paths() {
        let _ = pretty_env_logger::try_init();

        equal_tokens!(
            <pipeline>
            r#"sys | where cpu."max ghz" > 1"# -> b::pipeline(vec![
                    vec![
                        b::bare("sys"), b::sp()
                    ],
                    vec![
                        b::sp(),
                        b::bare("where"),
                        b::sp(),
                        b::bare("cpu"),
                        b::dot(),
                        b::string("max ghz"),
                        b::sp(),
                        b::op(">"),
                        b::sp(),
                        b::int(1)
                    ]])
        );
    }

    #[test]
    fn test_signature() {
        let _ = pretty_env_logger::try_init();

        equal_tokens!(
            <module>
            "def cd\n  # Change to a new path.\n  optional directory(Path) # the directory to change to\nend" ->
            b::token_list(vec![
                b::bare("def"),
                b::sp(),
                b::bare("cd"),
                b::sep("\n"),
                b::ws("  "),
                b::comment(" Change to a new path."),
                b::sep("\n"),
                b::ws("  "),
                b::bare("optional"),
                b::sp(),
                b::bare("directory"),
                b::parens(vec![b::bare("Path")]),
                b::sp(),
                b::comment(" the directory to change to"),
                b::sep("\n"),
                b::bare("end")
            ])
        );
    }

    // #[test]
    // fn test_smoke_pipeline() {
    //     let _ = pretty_env_logger::try_init();

    //     assert_eq!(
    //         apply(
    //             pipeline,
    //             "pipeline",
    //             r#"git branch --merged | split-row "`n" | where $it != "* master""#
    //         ),
    //         build_token(b::pipeline(vec![
    //             (
    //                 None,
    //                 b::call(
    //                     b::bare("git"),
    //                     vec![b::sp(), b::bare("branch"), b::sp(), b::flag("merged")]
    //                 ),
    //                 Some(" ")
    //             ),
    //             (
    //                 Some(" "),
    //                 b::call(b::bare("split-row"), vec![b::sp(), b::string("`n")]),
    //                 Some(" ")
    //             ),
    //             (
    //                 Some(" "),
    //                 b::call(
    //                     b::bare("where"),
    //                     vec![
    //                         b::sp(),
    //                         b::var("it"),
    //                         b::sp(),
    //                         b::op("!="),
    //                         b::sp(),
    //                         b::string("* master")
    //                     ]
    //                 ),
    //                 None
    //             )
    //         ]))
    //     );

    //     assert_eq!(
    //         apply(pipeline, "pipeline", "ls | where { $it.size > 100 }"),
    //         build_token(b::pipeline(vec![
    //             (None, b::call(b::bare("ls"), vec![]), Some(" ")),
    //             (
    //                 Some(" "),
    //                 b::call(
    //                     b::bare("where"),
    //                     vec![
    //                         b::sp(),
    //                         b::braced(vec![
    //                             b::path(b::var("it"), vec![b::member("size")]),
    //                             b::sp(),
    //                             b::op(">"),
    //                             b::sp(),
    //                             b::int(100)
    //                         ])
    //                     ]
    //                 ),
    //                 None
    //             )
    //         ]))
    //     )
    // }

    fn apply(
        f: impl Fn(NomSpan) -> Result<(NomSpan, TokenNode), nom::Err<(NomSpan, nom::error::ErrorKind)>>,
        desc: &str,
        string: &str,
    ) -> TokenNode {
        let result = f(nom_input(string));

        match result {
            Ok(value) => value.1,
            Err(err) => {
                let err = nu_errors::ShellError::parse_error(err);

                println!("{:?}", string);
                crate::hir::baseline_parse::tests::print_err(err, &nu_source::Text::from(string));
                panic!("test failed")
            }
        }
    }

    fn span((left, right): (usize, usize)) -> Span {
        Span::new(left, right)
    }

    fn delimited(
        delimiter: Spanned<Delimiter>,
        children: Vec<TokenNode>,
        left: usize,
        right: usize,
    ) -> TokenNode {
        let start = Span::for_char(left);
        let end = Span::for_char(right);

        let node = DelimitedNode::new(delimiter.item, (start, end), children);
        let spanned = node.spanned(Span::new(left, right));
        TokenNode::Delimited(spanned)
    }

    fn token(token: UnspannedToken, left: usize, right: usize) -> TokenNode {
        TokenNode::Token(token.into_token(Span::new(left, right)))
    }

    fn build<T>(block: CurriedNode<T>) -> T {
        let mut builder = TokenTreeBuilder::new();
        block(&mut builder)
    }

    fn build_token(block: CurriedToken) -> TokenNode {
        TokenTreeBuilder::build(block).0
    }
}
