use crate::parser::lexer::SpannedToken;
use crate::prelude::*;
use adhoc_derive::FromStr;
use derive_new::new;
use getset::Getters;
use serde_derive::{Deserialize, Serialize};
use std::io::Write;
use std::str::FromStr;

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

impl From<&str> for Operator {
    fn from(input: &str) -> Operator {
        Operator::from_str(input).unwrap()
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

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Expression {
    Leaf(Leaf),
    Flag(Flag),
    Parenthesized(Box<Parenthesized>),
    Block(Box<Block>),
    Binary(Box<Binary>),
    Path(Box<Path>),
    Call(Box<ParsedCommand>),
    VariableReference(Variable),
}

impl From<&str> for Expression {
    fn from(input: &str) -> Expression {
        Expression::Leaf(Leaf::String(input.into()))
    }
}

impl From<String> for Expression {
    fn from(input: String) -> Expression {
        Expression::Leaf(Leaf::String(input.into()))
    }
}

impl From<i64> for Expression {
    fn from(input: i64) -> Expression {
        Expression::Leaf(Leaf::Int(input.into()))
    }
}

impl From<BarePath> for Expression {
    fn from(input: BarePath) -> Expression {
        Expression::Leaf(Leaf::Bare(input))
    }
}

impl From<Variable> for Expression {
    fn from(input: Variable) -> Expression {
        Expression::VariableReference(input)
    }
}

impl From<Flag> for Expression {
    fn from(input: Flag) -> Expression {
        Expression::Flag(input)
    }
}

impl From<Binary> for Expression {
    fn from(input: Binary) -> Expression {
        Expression::Binary(Box::new(input))
    }
}

impl Expression {
    crate fn leaf(leaf: impl Into<Leaf>) -> Expression {
        Expression::Leaf(leaf.into())
    }

    crate fn flag(flag: impl Into<Flag>) -> Expression {
        Expression::Flag(flag.into())
    }

    crate fn call(head: Expression, tail: Vec<Expression>) -> Expression {
        if tail.len() == 0 {
            Expression::Call(Box::new(ParsedCommand::new(head.into(), None)))
        } else {
            Expression::Call(Box::new(ParsedCommand::new(head.into(), Some(tail))))
        }
    }

    crate fn binary(
        left: impl Into<Expression>,
        operator: impl Into<Operator>,
        right: impl Into<Expression>,
    ) -> Expression {
        Expression::Binary(Box::new(Binary {
            left: left.into(),
            operator: operator.into(),
            right: right.into(),
        }))
    }

    crate fn block(expr: impl Into<Expression>) -> Expression {
        Expression::Block(Box::new(Block::new(expr.into())))
    }

    crate fn print(&self) -> String {
        match self {
            Expression::Call(c) => c.print(),
            Expression::Leaf(l) => l.print(),
            Expression::Flag(f) => f.print(),
            Expression::Parenthesized(p) => p.print(),
            Expression::Block(b) => b.print(),
            Expression::VariableReference(r) => r.print(),
            Expression::Path(p) => p.print(),
            Expression::Binary(b) => b.print(),
        }
    }

    crate fn as_external_arg(&self) -> String {
        match self {
            Expression::Call(c) => c.as_external_arg(),
            Expression::Leaf(l) => l.as_external_arg(),
            Expression::Flag(f) => f.as_external_arg(),
            Expression::Parenthesized(p) => p.as_external_arg(),
            Expression::Block(b) => b.as_external_arg(),
            Expression::VariableReference(r) => r.as_external_arg(),
            Expression::Path(p) => p.as_external_arg(),
            Expression::Binary(b) => b.as_external_arg(),
        }
    }

    crate fn bare(path: impl Into<BarePath>) -> Expression {
        Expression::Leaf(Leaf::Bare(path.into()))
    }

    crate fn as_string(&self) -> Option<String> {
        match self {
            Expression::Leaf(Leaf::String(s)) => Some(s.to_string()),
            Expression::Leaf(Leaf::Bare(path)) => Some(path.to_string()),
            _ => None,
        }
    }

    crate fn as_bare(&self) -> Option<String> {
        match self {
            Expression::Leaf(Leaf::Bare(p)) => Some(p.to_string()),
            _ => None,
        }
    }

    crate fn is_flag(&self, value: &str) -> bool {
        match self {
            Expression::Flag(Flag::Longhand(f)) if value == f => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, new)]
pub struct Block {
    crate expr: Expression,
}

impl Block {
    fn print(&self) -> String {
        format!("{{ {} }}", self.expr.print())
    }

    fn as_external_arg(&self) -> String {
        format!("{{ {} }}", self.expr.as_external_arg())
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, new)]
pub struct Parenthesized {
    crate expr: Expression,
}

impl Parenthesized {
    fn print(&self) -> String {
        format!("({})", self.expr.print())
    }

    fn as_external_arg(&self) -> String {
        format!("({})", self.expr.as_external_arg())
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Getters, new)]
pub struct Path {
    #[get = "crate"]
    head: Expression,

    #[get = "crate"]
    tail: Vec<String>,
}

impl Path {
    crate fn print(&self) -> String {
        let mut out = self.head.print();

        for item in self.tail.iter() {
            out.push_str(&format!(".{}", item));
        }

        out
    }

    crate fn as_external_arg(&self) -> String {
        let mut out = self.head.as_external_arg();

        for item in self.tail.iter() {
            out.push_str(&format!(".{}", item));
        }

        out
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Variable {
    It,
    Other(String),
}

#[cfg(test)]
crate fn var(name: &str) -> Expression {
    match name {
        "it" => Expression::VariableReference(Variable::It),
        other => Expression::VariableReference(Variable::Other(other.to_string())),
    }
}

impl Variable {
    crate fn from_str(input: &str) -> Expression {
        match input {
            "it" => Expression::VariableReference(Variable::It),
            "yes" => Expression::Leaf(Leaf::Boolean(true)),
            "no" => Expression::Leaf(Leaf::Boolean(false)),
            other => Expression::VariableReference(Variable::Other(other.to_string())),
        }
    }

    fn print(&self) -> String {
        match self {
            Variable::It => format!("$it"),
            Variable::Other(s) => format!("${}", s),
        }
    }

    fn as_external_arg(&self) -> String {
        self.print()
    }
}

pub fn bare(s: impl Into<String>) -> BarePath {
    BarePath {
        head: s.into(),
        tail: vec![],
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct BarePath {
    head: String,
    tail: Vec<String>,
}

impl<T: Into<String>> From<T> for BarePath {
    fn from(input: T) -> BarePath {
        BarePath {
            head: input.into(),
            tail: vec![],
        }
    }
}

impl BarePath {
    crate fn from_token(head: SpannedToken) -> BarePath {
        BarePath {
            head: head.to_string(),
            tail: vec![],
        }
    }

    crate fn from_tokens(head: SpannedToken, tail: Vec<SpannedToken>) -> BarePath {
        BarePath {
            head: head.to_string(),
            tail: tail.iter().map(|i| i.to_string()).collect(),
        }
    }

    crate fn to_string(&self) -> String {
        bare_string(&self.head, &self.tail)
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, FromStr)]
pub enum Unit {
    #[adhoc(regex = "^B$")]
    B,
    #[adhoc(regex = "^KB$")]
    KB,
    #[adhoc(regex = "^MB$")]
    MB,
    #[adhoc(regex = "^GB$")]
    GB,
    #[adhoc(regex = "^TB$")]
    TB,
    #[adhoc(regex = "^PB$")]
    PB,
}

impl From<&str> for Unit {
    fn from(input: &str) -> Unit {
        Unit::from_str(input).unwrap()
    }
}

impl Unit {
    crate fn compute(&self, size: i64) -> Value {
        Value::int(match self {
            Unit::B => size,
            Unit::KB => size * 1024,
            Unit::MB => size * 1024 * 1024,
            Unit::GB => size * 1024 * 1024 * 1024,
            Unit::TB => size * 1024 * 1024 * 1024 * 1024,
            Unit::PB => size * 1024 * 1024 * 1024 * 1024 * 1024,
        })
    }
}

#[cfg(test)]
pub fn unit(num: i64, unit: impl Into<Unit>) -> Expression {
    Expression::Leaf(Leaf::Unit(num, unit.into()))
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Leaf {
    String(String),
    Bare(BarePath),

    #[allow(unused)]
    Boolean(bool),
    Int(i64),
    Unit(i64, Unit),
}

crate fn bare_string(head: &String, tail: &Vec<String>) -> String {
    let mut out = vec![head.clone()];
    out.extend(tail.clone());
    itertools::join(out, ".")
}

impl Leaf {
    fn print(&self) -> String {
        match self {
            Leaf::String(s) => format!("{:?}", s),
            Leaf::Bare(path) => format!("{}", path.to_string()),
            Leaf::Boolean(b) => format!("{}", b),
            Leaf::Int(i) => format!("{}", i),
            Leaf::Unit(i, unit) => format!("{}{:?}", i, unit),
        }
    }

    fn as_external_arg(&self) -> String {
        match self {
            Leaf::String(s) => format!("{}", s),
            Leaf::Bare(path) => format!("{}", path.to_string()),
            Leaf::Boolean(b) => format!("{}", b),
            Leaf::Int(i) => format!("{}", i),
            Leaf::Unit(i, unit) => format!("{}{:?}", i, unit),
        }
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Binary {
    crate left: Expression,
    crate operator: Operator,
    crate right: Expression,
}

impl Binary {
    crate fn new(
        left: impl Into<Expression>,
        operator: Operator,
        right: impl Into<Expression>,
    ) -> Binary {
        Binary {
            left: left.into(),
            operator,
            right: right.into(),
        }
    }
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

    fn as_external_arg(&self) -> String {
        format!(
            "{} {} {}",
            self.left.as_external_arg(),
            self.operator.print(),
            self.right.as_external_arg()
        )
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Flag {
    Shorthand(String),
    Longhand(String),
}

#[cfg(test)]
crate fn flag(s: &str) -> Flag {
    Flag::Longhand(s.into())
}

#[cfg(test)]
crate fn short(s: &str) -> Flag {
    Flag::Shorthand(s.into())
}

impl Flag {
    #[allow(unused)]
    crate fn print(&self) -> String {
        match self {
            Flag::Shorthand(s) => format!("-{}", s),
            Flag::Longhand(s) => format!("--{}", s),
        }
    }

    #[allow(unused)]
    crate fn as_external_arg(&self) -> String {
        self.print()
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, new)]
pub struct ParsedCommand {
    crate name: Expression,
    crate args: Option<Vec<Expression>>,
}

impl ParsedCommand {
    fn as_external_arg(&self) -> String {
        let mut out = vec![];

        write!(out, "{}", self.name.as_external_arg()).unwrap();

        if let Some(args) = &self.args {
            for arg in args.iter() {
                write!(out, " {}", arg.as_external_arg()).unwrap();
            }
        }

        String::from_utf8_lossy(&out).into_owned()
    }

    fn print(&self) -> String {
        let mut out = vec![];

        write!(out, "{}", self.name.print()).unwrap();

        if let Some(args) = &self.args {
            for arg in args.iter() {
                write!(out, " {}", arg.print()).unwrap();
            }
        }

        String::from_utf8_lossy(&out).into_owned()
    }
}

impl From<&str> for ParsedCommand {
    fn from(input: &str) -> ParsedCommand {
        ParsedCommand {
            name: Expression::Leaf(Leaf::Bare(bare(input))),
            args: None,
        }
    }
}

impl From<(&str, Vec<Expression>)> for ParsedCommand {
    fn from(input: (&str, Vec<Expression>)) -> ParsedCommand {
        ParsedCommand {
            name: Expression::bare(input.0),
            args: Some(input.1),
        }
    }
}

#[derive(new, Debug, Eq, PartialEq)]
pub struct Pipeline {
    crate commands: Vec<Expression>,
}

impl Pipeline {
    crate fn from_parts(command: Expression, rest: Vec<Expression>) -> Pipeline {
        let mut commands = vec![command];
        commands.extend(rest);

        Pipeline { commands }
    }

    #[allow(unused)]
    crate fn print(&self) -> String {
        itertools::join(self.commands.iter().map(|i| i.print()), " | ")
    }
}
