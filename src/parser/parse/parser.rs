#![allow(unused)]

use crate::parser::parse::{
    call_node::*, flag::*, operator::*, pipeline::*, token_tree::*, token_tree_builder::*,
    tokens::*, unit::*,
};
use crate::prelude::*;
use crate::{Tag, Tagged};
use nom;
use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::multi::*;
use nom::sequence::*;

use derive_new::new;
use log::trace;
use nom::dbg;
use nom::*;
use nom::{AsBytes, FindSubstring, IResult, InputLength, InputTake, Slice};
use nom_locate::{position, LocatedSpanEx};
use nom_tracable::{tracable_parser, HasTracableInfo, TracableInfo};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::str::FromStr;

pub type NomSpan<'a> = LocatedSpanEx<&'a str, TracableContext>;

#[derive(Debug, Clone, Copy, PartialEq, new)]
pub struct TracableContext {
    pub(crate) info: TracableInfo,
}

impl HasTracableInfo for TracableContext {
    fn get_tracable_info(&self) -> TracableInfo {
        self.info
    }

    fn set_tracable_info(mut self, info: TracableInfo) -> Self {
        TracableContext { info }
    }
}

impl std::ops::Deref for TracableContext {
    type Target = TracableInfo;

    fn deref(&self) -> &TracableInfo {
        &self.info
    }
}

pub fn nom_input(s: &str) -> NomSpan<'_> {
    LocatedSpanEx::new_extra(s, TracableContext::new(TracableInfo::new()))
}

macro_rules! operator {
    ($name:tt : $token:tt ) => {
        #[tracable_parser]
        pub fn $name(input: NomSpan) -> IResult<NomSpan, TokenNode> {
            let start = input.offset;
            let (input, tag) = tag(stringify!($token))(input)?;
            let end = input.offset;

            Ok((
                input,
                TokenTreeBuilder::spanned_op(tag.fragment, Span::new(start, end)),
            ))
        }
    };
}

operator! { gt:  >  }
operator! { lt:  <  }
operator! { gte: >= }
operator! { lte: <= }
operator! { eq:  == }
operator! { neq: != }
operator! { dot: . }

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Number {
    Int(BigInt),
    Decimal(BigDecimal),
}

impl std::fmt::Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::Int(int) => write!(f, "{}", int),
            Number::Decimal(decimal) => write!(f, "{}", decimal),
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
                    Number::Decimal(BigDecimal::$from(decimal).unwrap())
                }
            }

            impl From<&$ty> for Number {
                fn from(decimal: &$ty) -> Number {
                    Number::Decimal(BigDecimal::$from(*decimal).unwrap())
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
        TokenTreeBuilder::spanned_number(number.item, number.span),
    ))
}

#[tracable_parser]
pub fn raw_number(input: NomSpan) -> IResult<NomSpan, Spanned<RawNumber>> {
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
    let (input, operator) = alt((gte, lte, neq, gt, lt, eq))(input)?;

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
    let (input, bare) = take_while(is_bare_char)(input)?;
    let end = input.offset;

    Ok((
        input,
        TokenTreeBuilder::spanned_external_command(bare, Span::new(start, end)),
    ))
}

#[tracable_parser]
pub fn pattern(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let start = input.offset;
    let (input, _) = take_while1(is_start_glob_char)(input)?;
    let (input, _) = take_while(is_glob_char)(input)?;

    let next_char = &input.fragment.chars().nth(0);

    if let Some(next_char) = next_char {
        if is_external_word_char(*next_char) {
            return Err(nom::Err::Error(nom::error::make_error(
                input,
                nom::error::ErrorKind::TakeWhile1,
            )));
        }
    }

    let end = input.offset;

    Ok((
        input,
        TokenTreeBuilder::spanned_pattern(Span::new(start, end)),
    ))
}

#[tracable_parser]
pub fn bare(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let start = input.offset;
    let (input, _) = take_while1(is_start_bare_char)(input)?;
    let (input, last) = take_while(is_bare_char)(input)?;

    let next_char = &input.fragment.chars().nth(0);
    let prev_char = last.fragment.chars().nth(0);

    if let Some(next_char) = next_char {
        if is_external_word_char(*next_char) || is_glob_specific_char(*next_char) {
            return Err(nom::Err::Error(nom::error::make_error(
                input,
                nom::error::ErrorKind::TakeWhile1,
            )));
        }
    }

    let end = input.offset;

    Ok((input, TokenTreeBuilder::spanned_bare(Span::new(start, end))))
}

#[tracable_parser]
pub fn external_word(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let start = input.offset;
    let (input, _) = take_while1(is_external_word_char)(input)?;
    let end = input.offset;

    Ok((
        input,
        TokenTreeBuilder::spanned_external_word(Span::new(start, end)),
    ))
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

#[tracable_parser]
pub fn ident(input: NomSpan) -> IResult<NomSpan, Tag> {
    let start = input.offset;
    let (input, _) = take_while1(is_start_bare_char)(input)?;
    let (input, _) = take_while(is_bare_char)(input)?;
    let end = input.offset;

    Ok((input, Tag::from((start, end, None))))
}

#[tracable_parser]
pub fn flag(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let start = input.offset;
    let (input, _) = tag("--")(input)?;
    let (input, bare) = bare(input)?;
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
    let (input, bare) = bare(input)?;
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
    let (input, first) = node(input)?;

    let (input, mut list) = many0(pair(alt((whitespace, dot)), node))(input)?;

    let end = input.offset;

    Ok((
        input,
        make_token_list(first, list, None).spanned(Span::new(start, end)),
    ))
}

#[tracable_parser]
pub fn spaced_token_list(input: NomSpan) -> IResult<NomSpan, Spanned<Vec<TokenNode>>> {
    let start = input.offset;
    let (input, pre_ws) = opt(whitespace)(input)?;
    let (input, items) = token_list(input)?;
    let (input, post_ws) = opt(whitespace)(input)?;
    let end = input.offset;

    let mut out = vec![];

    out.extend(pre_ws);
    out.extend(items.item);
    out.extend(post_ws);

    Ok((input, out.spanned(Span::new(start, end))))
}

fn make_token_list(
    first: Vec<TokenNode>,
    list: Vec<(TokenNode, Vec<TokenNode>)>,
    sp_right: Option<TokenNode>,
) -> Vec<TokenNode> {
    let mut nodes = vec![];

    nodes.extend(first);

    for (left, right) in list {
        nodes.push(left);
        nodes.extend(right);
    }

    if let Some(sp_right) = sp_right {
        nodes.push(sp_right);
    }

    nodes
}

#[tracable_parser]
pub fn whitespace(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    let left = input.offset;
    let (input, ws1) = space1(input)?;
    let right = input.offset;

    Ok((input, TokenTreeBuilder::spanned_ws(Span::new(left, right))))
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
pub fn bare_path(input: NomSpan) -> IResult<NomSpan, Vec<TokenNode>> {
    let (input, head) = alt((bare, dot))(input)?;

    let (input, tail) = many0(alt((bare, dot, string)))(input)?;

    let next_char = &input.fragment.chars().nth(0);

    if is_boundary(*next_char) {
        let mut result = vec![head];
        result.extend(tail);

        Ok((input, result))
    } else {
        Err(nom::Err::Error(nom::error::make_error(
            input,
            nom::error::ErrorKind::Many0,
        )))
    }
}

#[tracable_parser]
pub fn pattern_path(input: NomSpan) -> IResult<NomSpan, Vec<TokenNode>> {
    let (input, head) = alt((pattern, dot))(input)?;

    let (input, tail) = many0(alt((pattern, dot, string)))(input)?;

    let next_char = &input.fragment.chars().nth(0);

    if is_boundary(*next_char) {
        let mut result = vec![head];
        result.extend(tail);

        Ok((input, result))
    } else {
        Err(nom::Err::Error(nom::error::make_error(
            input,
            nom::error::ErrorKind::Many0,
        )))
    }
}

#[tracable_parser]
pub fn node1(input: NomSpan) -> IResult<NomSpan, TokenNode> {
    alt((leaf, bare, pattern, external_word, delimited_paren))(input)
}

#[tracable_parser]
pub fn node(input: NomSpan) -> IResult<NomSpan, Vec<TokenNode>> {
    alt((
        to_list(leaf),
        bare_path,
        pattern_path,
        to_list(external_word),
        to_list(delimited_paren),
        to_list(delimited_brace),
        to_list(delimited_square),
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
    let mut all_items: Vec<Spanned<PipelineElement>> =
        vec![PipelineElement::new(None, head).spanned(head_span)];

    all_items.extend(items.into_iter().map(|(pipe, items)| {
        let items_span = items.span;
        PipelineElement::new(Some(Span::from(pipe)), items)
            .spanned(Span::from(pipe).until(items_span))
    }));

    Ok((
        input,
        TokenTreeBuilder::spanned_pipeline(all_items, Span::new(start, end)),
    ))
}

fn int<T>(frag: &str, neg: Option<T>) -> i64 {
    let int = FromStr::from_str(frag).unwrap();

    match neg {
        None => int,
        Some(_) => int * -1,
    }
}

fn is_boundary(c: Option<char>) -> bool {
    match c {
        None => true,
        Some(')') | Some(']') | Some('}') => true,
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
    is_start_bare_char(c) || is_glob_specific_char(c) || c == '.'
}

fn is_glob_char(c: char) -> bool {
    is_bare_char(c) || is_glob_specific_char(c)
}

fn is_start_bare_char(c: char) -> bool {
    match c {
        '+' => false,
        _ if c.is_alphanumeric() => true,
        '\\' => true,
        '/' => true,
        '_' => true,
        '-' => true,
        '~' => true,
        _ => false,
    }
}

fn is_bare_char(c: char) -> bool {
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
    use crate::parser::parse::token_tree_builder::TokenTreeBuilder as b;
    use crate::parser::parse::token_tree_builder::{CurriedToken, TokenTreeBuilder};
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
    fn test_operator() {
        equal_tokens! {
            <nodes>
            ">" -> b::token_list(vec![b::op(">")])
        }

        equal_tokens! {
            <nodes>
            ">=" -> b::token_list(vec![b::op(">=")])
        }

        equal_tokens! {
            <nodes>
            "<" -> b::token_list(vec![b::op("<")])
        }

        equal_tokens! {
            <nodes>
            "<=" -> b::token_list(vec![b::op("<=")])
        }

        equal_tokens! {
            <nodes>
            "==" -> b::token_list(vec![b::op("==")])
        }

        equal_tokens! {
            <nodes>
            "!=" -> b::token_list(vec![b::op("!=")])
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
            "chrome.exe" -> b::token_list(vec![b::bare("chrome"), b::op(Operator::Dot), b::bare("exe")])
        }

        equal_tokens! {
            <nodes>
            ".azure" -> b::token_list(vec![b::op(Operator::Dot), b::bare("azure")])
        }

        equal_tokens! {
            <nodes>
            r"C:\windows\system.dll" -> b::token_list(vec![b::bare(r"C:\windows\system"), b::op(Operator::Dot), b::bare("dll")])
        }

        equal_tokens! {
            <nodes>
            r"C:\Code\-testing\my_tests.js" -> b::token_list(vec![b::bare(r"C:\Code\-testing\my_tests"), b::op(Operator::Dot), b::bare("js")])
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
            ".azure" -> b::token_list(vec![b::op("."), b::bare("azure")])
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
    fn test_path() {
        let _ = pretty_env_logger::try_init();

        equal_tokens! {
            <nodes>
            "$it.print" -> b::token_list(vec![b::var("it"), b::op("."), b::bare("print")])
        }

        equal_tokens! {
            <nodes>
            "$it.0" -> b::token_list(vec![b::var("it"), b::op("."), b::int(0)])
        }

        equal_tokens! {
            <nodes>
            "$head.part1.part2" -> b::token_list(vec![b::var("head"), b::op("."), b::bare("part1"), b::op("."), b::bare("part2")])
        }

        equal_tokens! {
            <nodes>
            "( hello ).world" -> b::token_list(vec![b::parens(vec![b::sp(), b::bare("hello"), b::sp()]), b::op("."), b::bare("world")])
        }

        equal_tokens! {
            <nodes>
            r#"( hello )."world""# -> b::token_list(vec![b::parens(vec![b::sp(), b::bare("hello"), b::sp()]), b::op("."), b::string("world")])
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
                        b::op("."),
                        b::bare("is"),
                        b::op("."),
                        b::string("great news"),
                        b::op("."),
                        b::bare("right"),
                        b::sp(),
                        b::bare("yep"),
                        b::sp(),
                        b::var("yep"),
                        b::sp()
                    ]),
                    b::op("."), b::string("world")]
            )
        }

        equal_tokens! {
            <nodes>
            r#"$it."are PAS".0"# -> b::token_list(
                vec![
                    b::var("it"),
                    b::op("."),
                    b::string("are PAS"),
                    b::op("."),
                    b::int(0),
                    ]
            )
        }
    }

    #[test]
    fn test_smoke_single_command() {
        equal_tokens! {
            <nodes>
            "git add ." -> b::token_list(vec![b::bare("git"), b::sp(), b::bare("add"), b::sp(), b::op(".")])
        }

        equal_tokens! {
            <nodes>
            "open Cargo.toml" -> b::token_list(vec![b::bare("open"), b::sp(), b::bare("Cargo"), b::op("."), b::bare("toml")])
        }

        equal_tokens! {
            <nodes>
            "select package.version" -> b::token_list(vec![b::bare("select"), b::sp(), b::bare("package"), b::op("."), b::bare("version")])
        }

        equal_tokens! {
            <nodes>
            "echo $it" -> b::token_list(vec![b::bare("echo"), b::sp(), b::var("it")])
        }

        equal_tokens! {
            <nodes>
            "open Cargo.toml --raw" -> b::token_list(vec![b::bare("open"), b::sp(), b::bare("Cargo"), b::op("."), b::bare("toml"), b::sp(), b::flag("raw")])
        }

        equal_tokens! {
            <nodes>
            "open Cargo.toml -r" -> b::token_list(vec![b::bare("open"), b::sp(), b::bare("Cargo"), b::op("."), b::bare("toml"), b::sp(), b::shorthand("r")])
        }

        equal_tokens! {
            <nodes>
            "config --set tabs 2" -> b::token_list(vec![b::bare("config"), b::sp(), b::flag("set"), b::sp(), b::bare("tabs"), b::sp(), b::int(2)])
        }

        equal_tokens! {
            <nodes>
            "inc --patch package.version" -> b::token_list(
                vec![
                    b::bare("inc"),
                    b::sp(),
                    b::flag("patch"),
                    b::sp(),
                    b::bare("package"), b::op("."), b::bare("version")
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
                        b::op("."),
                        b::string("max ghz"),
                        b::sp(),
                        b::op(">"),
                        b::sp(),
                        b::int(1)
                    ]])
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
        f(nom_input(string)).unwrap().1
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

    fn token(token: RawToken, left: usize, right: usize) -> TokenNode {
        TokenNode::Token(token.spanned(Span::new(left, right)))
    }

    fn build<T>(block: CurriedNode<T>) -> T {
        let mut builder = TokenTreeBuilder::new();
        block(&mut builder)
    }

    fn build_token(block: CurriedToken) -> TokenNode {
        TokenTreeBuilder::build(block).0
    }
}
