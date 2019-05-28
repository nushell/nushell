use derive_new::new;
use std::str::FromStr;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
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

#[derive(Debug, Clone)]
pub enum Expression {
    Leaf(Leaf),
    Binary(Binary),
}

impl Expression {
    crate fn print(&self) -> String {
        match self {
            Expression::Leaf(l) => l.print(),
            Expression::Binary(b) => b.print(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Leaf {
    String(String),
    Bare(String),
    Boolean(bool),
    Int(i64),
}

impl Leaf {
    fn print(&self) -> String {
        match self {
            Leaf::String(s) => format!("{:?}", s),
            Leaf::Bare(s) => format!("{}", s),
            Leaf::Boolean(b) => format!("{}", b),
            Leaf::Int(i) => format!("{}", i),
        }
    }
}

#[derive(Debug, Clone, new)]
pub struct Binary {
    crate left: Leaf,
    crate operator: Operator,
    crate right: Leaf,
}

impl Binary {
    fn print(&self) -> String {
        format!(
            "{} {} {}",
            self.left.print(),
            self.operator.print(),
            self.right.print()
        )
    }
}

#[derive(Debug, Clone)]
pub enum Flag {
    Shorthand(String),
    Longhand(String),
}

impl Flag {
    #[allow(unused)]
    fn print(&self) -> String {
        match self {
            Flag::Shorthand(s) => format!("-{}", s),
            Flag::Longhand(s) => format!("--{}", s),
        }
    }
}

#[derive(new, Debug, Clone)]
pub struct ParsedCommand {
    crate name: String,
    crate args: Vec<Expression>,
}

#[derive(new, Debug)]
pub struct Pipeline {
    crate commands: Vec<ParsedCommand>,
}

impl Pipeline {
    crate fn from_parts(command: ParsedCommand, rest: Vec<ParsedCommand>) -> Pipeline {
        let mut commands = vec![command];
        commands.extend(rest);

        Pipeline { commands }
    }
}
