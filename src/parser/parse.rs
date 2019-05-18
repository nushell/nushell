use crate::prelude::*;
use nom::branch::alt;
use nom::bytes::complete::{escaped, is_a, is_not, tag};
use nom::character::complete::one_of;
use nom::multi::separated_list;
use nom::sequence::{preceded, terminated};
use nom::IResult;
use nom::{complete, named, ws};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum Item {
    Quoted(String),
    Bare(String),
    Int(i64),
    Boolean(bool),
    Operator(Operator),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Operator {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

impl Operator {
    pub fn print(&self) -> String {
        match *self {
            Operator::Equal => "==".to_string(),
            Operator::NotEqual => "!=".to_string(),
            Operator::LessThan => "<".to_string(),
            Operator::GreaterThan => ">".to_string(),
            Operator::LessThanOrEqual => "<=".to_string(),
            Operator::GreaterThanOrEqual => ">=".to_string(),
        }
    }
}

impl FromStr for Operator {
    type Err = ();
    fn from_str(input: &str) -> Result<Self, <Self as std::str::FromStr>::Err> {
        match input {
            "==" => Ok(Operator::Equal),
            "!=" => Ok(Operator::NotEqual),
            "<" => Ok(Operator::LessThan),
            ">" => Ok(Operator::GreaterThan),
            "<=" => Ok(Operator::LessThanOrEqual),
            ">=" => Ok(Operator::GreaterThanOrEqual),
            _ => Err(()),
        }
    }
}

impl Item {
    crate fn as_value(&self) -> Value {
        match self {
            Item::Quoted(s) => Value::Primitive(Primitive::String(s.clone())),
            Item::Bare(s) => Value::Primitive(Primitive::String(s.clone())),
            Item::Int(i) => Value::Primitive(Primitive::Int(*i)),
            Item::Boolean(b) => Value::Primitive(Primitive::Boolean(*b)),
            Item::Operator(o) => Value::Primitive(Primitive::Operator(o.clone())),
        }
    }

    pub fn print(&self) -> String {
        match self {
            Item::Bare(s) => format!("{}", s),
            Item::Quoted(s) => format!("{}", s),
            Item::Int(i) => format!("{:?}", i),
            Item::Boolean(b) => format!("{:?}", b),
            Item::Operator(o) => o.print(),
        }
    }
}

impl Item {
    crate fn name(&self) -> Result<&str, ShellError> {
        match self {
            Item::Quoted(s) => Ok(s),
            Item::Bare(s) => Ok(s),
            Item::Boolean(i) => Err(ShellError::string(format!("{} is not a valid command", i))),
            Item::Int(i) => Err(ShellError::string(format!("{} is not a valid command", i))),
            Item::Operator(x) => Err(ShellError::string(format!(
                "{:?} is not a valid command",
                x
            ))),
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

fn operator(s: &str) -> IResult<&str, Item> {
    alt((
        tag("=="),
        tag("!="),
        tag("<"),
        tag(">"),
        tag("<="),
        tag(">="),
    ))(s)
    .map(|(a, b)| (a, Item::Operator(FromStr::from_str(b).unwrap())))
}

fn int(s: &str) -> IResult<&str, Item> {
    is_a("1234567890")(s).map(|(a, b)| (a, Item::Int(FromStr::from_str(b).unwrap())))
}

fn boolean(s: &str) -> IResult<&str, Item> {
    alt((tag("true"), tag("false")))(s)
        .map(|(a, b)| (a, Item::Boolean(FromStr::from_str(b).unwrap())))
}

fn command_token(s: &str) -> IResult<&str, Item> {
    alt((boolean, int, operator, quoted, unquoted))(s)
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
