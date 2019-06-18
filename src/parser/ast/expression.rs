use crate::parser::lexer::{Span, Spanned};
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
        self.as_str().to_string()
    }

    pub fn as_str(&self) -> &str {
        match *self {
            Operator::Equal => "==",
            Operator::NotEqual => "!=",
            Operator::LessThan => "<",
            Operator::GreaterThan => ">",
            Operator::LessThanOrEqual => "<=",
            Operator::GreaterThanOrEqual => ">=",
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
pub struct Expression {
    crate expr: RawExpression,
    crate span: Span,
}

impl std::ops::Deref for Expression {
    type Target = RawExpression;

    fn deref(&self) -> &RawExpression {
        &self.expr
    }
}

impl Expression {
    crate fn print(&self) -> String {
        self.expr.print()
    }

    crate fn as_external_arg(&self) -> String {
        self.expr.as_external_arg()
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum RawExpression {
    Leaf(Leaf),
    Flag(Flag),
    Parenthesized(Box<Parenthesized>),
    Block(Box<Block>),
    Binary(Box<Binary>),
    Path(Box<Path>),
    Call(Box<Call>),
    VariableReference(Variable),
}

impl RawExpression {
    crate fn print(&self) -> String {
        match self {
            RawExpression::Call(c) => c.print(),
            RawExpression::Leaf(l) => l.print(),
            RawExpression::Flag(f) => f.print(),
            RawExpression::Parenthesized(p) => p.print(),
            RawExpression::Block(b) => b.print(),
            RawExpression::VariableReference(r) => r.print(),
            RawExpression::Path(p) => p.print(),
            RawExpression::Binary(b) => b.print(),
        }
    }

    crate fn as_external_arg(&self) -> String {
        match self {
            RawExpression::Call(c) => c.as_external_arg(),
            RawExpression::Leaf(l) => l.as_external_arg(),
            RawExpression::Flag(f) => f.as_external_arg(),
            RawExpression::Parenthesized(p) => p.as_external_arg(),
            RawExpression::Block(b) => b.as_external_arg(),
            RawExpression::VariableReference(r) => r.as_external_arg(),
            RawExpression::Path(p) => p.as_external_arg(),
            RawExpression::Binary(b) => b.as_external_arg(),
        }
    }

    crate fn as_string(&self) -> Option<String> {
        match self {
            RawExpression::Leaf(Leaf::String(s)) => Some(s.to_string()),
            RawExpression::Leaf(Leaf::Bare(path)) => Some(path.to_string()),
            _ => None,
        }
    }

    #[allow(unused)]
    crate fn as_bare(&self) -> Option<String> {
        match self {
            RawExpression::Leaf(Leaf::Bare(p)) => Some(p.to_string()),
            _ => None,
        }
    }

    #[allow(unused)]
    crate fn as_block(&self) -> Option<Block> {
        match self {
            RawExpression::Block(block) => Some(*block.clone()),
            _ => None,
        }
    }

    crate fn is_flag(&self, value: &str) -> bool {
        match self {
            RawExpression::Flag(Flag::Longhand(f)) if value == f => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, new)]
pub struct Block {
    crate expr: Expression,
}

impl Block {
    crate fn print(&self) -> String {
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
    tail: Vec<Spanned<String>>,
}

impl Path {
    crate fn print(&self) -> String {
        let mut out = self.head.print();

        for item in self.tail.iter() {
            out.push_str(&format!(".{}", item.item));
        }

        out
    }

    crate fn as_external_arg(&self) -> String {
        let mut out = self.head.as_external_arg();

        for item in self.tail.iter() {
            out.push_str(&format!(".{}", item.item));
        }

        out
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Variable {
    It,
    Other(String),
}

impl Variable {
    crate fn from_string(s: &str) -> Variable {
        match s {
            "it" => ast::Variable::It,
            _ => ast::Variable::Other(s.to_string()),
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

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, new, Getters)]
pub struct Bare {
    #[get = "crate"]
    body: String,
}

impl From<String> for Bare {
    fn from(input: String) -> Bare {
        Bare { body: input }
    }
}

impl From<&str> for Bare {
    fn from(input: &str) -> Bare {
        Bare {
            body: input.to_string(),
        }
    }
}

impl Bare {
    crate fn from_string(string: impl Into<String>) -> Bare {
        Bare {
            body: string.into(),
        }
    }

    crate fn to_string(&self) -> String {
        self.body.to_string()
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

    crate fn to_string(&self) -> &str {
        match self {
            Unit::B => "B",
            Unit::KB => "KB",
            Unit::MB => "MB",
            Unit::GB => "GB",
            Unit::TB => "TB",
            Unit::PB => "PB",
        }
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum RawParameterIdentifier {
    #[allow(unused)]
    Bare(Spanned<Bare>),
    Var(Spanned<Variable>),
    ShorthandFlag(Spanned<String>),
    LonghandFlag(Spanned<String>),
}

impl RawParameterIdentifier {
    #[allow(unused)]
    pub fn print(&self) -> String {
        match self {
            RawParameterIdentifier::Bare(b) => b.to_string(),
            RawParameterIdentifier::Var(v) => v.print(),
            RawParameterIdentifier::ShorthandFlag(f) => f.to_string(),
            RawParameterIdentifier::LonghandFlag(f) => f.to_string(),
        }
    }
}

pub type ParameterIdentifier = Spanned<RawParameterIdentifier>;

impl ParameterIdentifier {
    #[allow(unused)]
    pub fn bare(bare: Spanned<Bare>, span: impl Into<Span>) -> ParameterIdentifier {
        let id = RawParameterIdentifier::Bare(bare);
        Spanned::from_item(id, span)
    }

    pub fn var(var: Spanned<Variable>, span: impl Into<Span>) -> ParameterIdentifier {
        let id = RawParameterIdentifier::Var(var);
        Spanned::from_item(id, span)
    }

    pub fn flag(flag: Spanned<String>, span: impl Into<Span>) -> ParameterIdentifier {
        let id = RawParameterIdentifier::LonghandFlag(flag);
        Spanned::from_item(id, span)
    }

    pub fn shorthand(flag: Spanned<String>, span: impl Into<Span>) -> ParameterIdentifier {
        let id = RawParameterIdentifier::ShorthandFlag(flag);
        Spanned::from_item(id, span)
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Leaf {
    String(String),
    Bare(Bare),
    Boolean(bool),
    Int(i64),
    Unit(i64, Unit),
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
            Leaf::String(s) => {
                #[cfg(windows)]
                {
                    format!("{}", s)
                }
                #[cfg(not(windows))]
                {
                    format!("\"{}\"", s)
                }
            }
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
    crate operator: Spanned<Operator>,
    crate right: Expression,
}

impl Binary {
    crate fn new(
        left: impl Into<Expression>,
        operator: Spanned<Operator>,
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
pub struct Call {
    crate name: Expression,
    crate args: Option<Vec<Expression>>,
}

impl From<(Expression, Vec<Expression>)> for Call {
    fn from(input: (Expression, Vec<Expression>)) -> Call {
        Call {
            name: input.0,
            args: if input.1.len() == 0 {
                None
            } else {
                Some(input.1)
            },
        }
    }
}

impl From<Expression> for Call {
    fn from(input: Expression) -> Call {
        Call {
            name: input,
            args: None,
        }
    }
}

impl Call {
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

#[derive(new, Debug, Eq, PartialEq, Clone)]
pub struct Pipeline {
    crate commands: Vec<Expression>,
    crate span: Span,
}

impl Pipeline {
    crate fn from_parts(
        command: Expression,
        rest: Vec<Expression>,
        start: usize,
        end: usize,
    ) -> Pipeline {
        let mut commands = vec![command];
        commands.extend(rest);

        Pipeline {
            commands,
            span: Span::from((start, end)),
        }
    }

    #[allow(unused)]
    crate fn print(&self) -> String {
        itertools::join(self.commands.iter().map(|i| i.print()), " | ")
    }
}
