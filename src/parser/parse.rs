use crate::prelude::*;
use nom::branch::alt;
use nom::bytes::complete::{escaped, is_a, is_not, tag};
use nom::character::complete::one_of;
use nom::multi::separated_list;
use nom::sequence::{preceded, terminated};
use nom::IResult;
use nom::{complete, named, separated_list, ws};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum Item {
    Quoted(String),
    Bare(String),
    Int(i64),
}

impl Item {
    crate fn as_value(&self) -> Value {
        match self {
            Item::Quoted(s) => Value::Primitive(Primitive::String(s.clone())),
            Item::Bare(s) => Value::Primitive(Primitive::String(s.clone())),
            Item::Int(i) => Value::Primitive(Primitive::Int(*i)),
        }
    }
}

crate fn print_items(items: &[Item]) -> String {
    let mut out = String::new();

    let formatted = items.iter().map(|item| match item {
        Item::Bare(s) => format!("{}", s),
        Item::Quoted(s) => format!("{:?}", s),
        Item::Int(i) => format!("{:?}", i),
    });

    itertools::join(formatted, " ")
}

impl Item {
    crate fn name(&self) -> &str {
        match self {
            Item::Quoted(s) => s,
            Item::Bare(s) => s,
            Item::Int(i) => unimplemented!(),
        }
    }
}

fn esc(s: &str) -> IResult<&str, &str> {
    escaped(is_not("\\\""), '\\', one_of("\"n\\"))(s)
}

fn quoted(s: &str) -> IResult<&str, Item> {
    terminated(preceded(tag("\""), esc), tag("\""))(s)
        .map(|(a, b)| (a, Item::Quoted(b.to_string())))
}

fn unquoted(s: &str) -> IResult<&str, Item> {
    is_not(" |")(s).map(|(a, b)| (a, Item::Bare(b.to_string())))
}

fn int(s: &str) -> IResult<&str, Item> {
    is_a("1234567890")(s).map(|(a, b)| (a, Item::Int(FromStr::from_str(b).unwrap())))
}

fn command_token(s: &str) -> IResult<&str, Item> {
    alt((int, quoted, unquoted))(s)
}

fn command_args(s: &str) -> IResult<&str, Vec<Item>> {
    separated_list(tag(" "), command_token)(s)
}

named!(
  pub shell_parser(&str) -> Vec<Vec<Item>>,
  complete!(
    ws!(
      separated_list!(tag("|"), command_args)
    )
  )
);
