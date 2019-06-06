use crate::parser::lexer::{Span, Spanned};
use crate::prelude::*;
use adhoc_derive::FromStr;
use derive_new::new;
use getset::Getters;
use serde_derive::{Deserialize, Serialize};
use std::io::Write;
use std::str::FromStr;

#[derive(new)]
pub struct ExpressionBuilder {
    #[new(default)]
    pos: usize,
}

#[allow(unused)]
impl ExpressionBuilder {
    pub fn op(&mut self, input: impl Into<Operator>) -> Spanned<Operator> {
        let input = input.into();

        let (start, end) = self.consume(input.as_str());

        self.pos = end;

        ExpressionBuilder::spanned_op(input, start, end)
    }

    pub fn spanned_op(input: impl Into<Operator>, start: usize, end: usize) -> Spanned<Operator> {
        Spanned {
            span: Span::from((start, end)),
            item: input.into(),
        }
    }

    pub fn string(&mut self, input: impl Into<String>) -> Expression {
        let input = input.into();

        let (start, _) = self.consume("\"");
        self.consume(&input);
        let (_, end) = self.consume("\"");
        self.pos = end;

        ExpressionBuilder::spanned_string(input, start, end)
    }

    pub fn spanned_string(input: impl Into<String>, start: usize, end: usize) -> Expression {
        let input = input.into();

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Leaf(Leaf::String(input)),
        }
    }

    pub fn bare(&mut self, input: impl Into<Bare>) -> Expression {
        let input = input.into();

        let (start, end) = self.consume(&input.body);
        self.pos = end;

        ExpressionBuilder::spanned_bare(input, start, end)
    }

    pub fn spanned_bare(input: impl Into<Bare>, start: usize, end: usize) -> Expression {
        let input = input.into();

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Leaf(Leaf::Bare(input)),
        }
    }

    pub fn boolean(&mut self, input: impl Into<bool>) -> Expression {
        let boolean = input.into();

        let (start, end) = match boolean {
            true => self.consume("$yes"),
            false => self.consume("$no"),
        };

        self.pos = end;

        ExpressionBuilder::spanned_boolean(boolean, start, end)
    }

    pub fn spanned_boolean(input: impl Into<bool>, start: usize, end: usize) -> Expression {
        let input = input.into();

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Leaf(Leaf::Boolean(input)),
        }
    }

    pub fn int(&mut self, input: impl Into<i64>) -> Expression {
        let int = input.into();

        let (start, end) = self.consume(&int.to_string());
        self.pos = end;

        ExpressionBuilder::spanned_int(int, start, end)
    }

    pub fn spanned_int(input: impl Into<i64>, start: usize, end: usize) -> Expression {
        let input = input.into();

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Leaf(Leaf::Int(input)),
        }
    }

    pub fn unit(&mut self, input: (impl Into<i64>, impl Into<Unit>)) -> Expression {
        let (int, unit) = (input.0.into(), input.1.into());

        let (start, _) = self.consume(&int.to_string());
        let (_, end) = self.consume(&unit.to_string());
        self.pos = end;

        ExpressionBuilder::spanned_unit((int, unit), start, end)
    }

    pub fn spanned_unit(
        input: (impl Into<i64>, impl Into<Unit>),
        start: usize,
        end: usize,
    ) -> Expression {
        let (int, unit) = (input.0.into(), input.1.into());

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Leaf(Leaf::Unit(int, unit)),
        }
    }

    pub fn flag(&mut self, input: impl Into<String>) -> Expression {
        let input = input.into();

        let (start, _) = self.consume("--");
        let (_, end) = self.consume(&input);
        self.pos = end;

        ExpressionBuilder::spanned_flag(input, start, end)
    }

    pub fn spanned_flag(input: impl Into<String>, start: usize, end: usize) -> Expression {
        let input = input.into();

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Flag(Flag::Longhand(input)),
        }
    }

    pub fn shorthand(&mut self, input: impl Into<String>) -> Expression {
        let int = input.into();

        let size = int.to_string().len();

        let start = self.pos;
        let end = self.pos + size + 1;
        self.pos = end;

        ExpressionBuilder::spanned_shorthand(int, start, end)
    }

    pub fn spanned_shorthand(input: impl Into<String>, start: usize, end: usize) -> Expression {
        let input = input.into();

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Flag(Flag::Shorthand(input)),
        }
    }

    pub fn parens(
        &mut self,
        input: impl FnOnce(&mut ExpressionBuilder) -> Expression,
    ) -> Expression {
        let (start, _) = self.consume("(");
        let input = input(self);
        let (_, end) = self.consume(")");
        self.pos = end;

        ExpressionBuilder::spanned_parens(input, start, end)
    }

    pub fn spanned_parens(input: Expression, start: usize, end: usize) -> Expression {
        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Parenthesized(Box::new(Parenthesized::new(input))),
        }
    }

    pub fn block(&mut self, input: &dyn Fn(&mut ExpressionBuilder) -> Expression) -> Expression {
        let (start, _) = self.consume("{ ");
        let input = input(self);
        let (_, end) = self.consume(" }");
        self.pos = end;

        ExpressionBuilder::spanned_block(input, start, end)
    }

    pub fn spanned_block(input: Expression, start: usize, end: usize) -> Expression {
        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Block(Box::new(Block::new(input))),
        }
    }

    pub fn binary(
        &mut self,
        input: (
            &dyn Fn(&mut ExpressionBuilder) -> Expression,
            &dyn Fn(&mut ExpressionBuilder) -> Spanned<Operator>,
            &dyn Fn(&mut ExpressionBuilder) -> Expression,
        ),
    ) -> Expression {
        let start = self.pos;

        let left = (input.0)(self);
        self.consume(" ");
        let operator = (input.1)(self);
        self.consume(" ");
        let right = (input.2)(self);

        let end = self.pos;

        ExpressionBuilder::spanned_binary((left, operator, right), start, end)
    }

    pub fn spanned_binary(
        input: (
            impl Into<Expression>,
            impl Into<Spanned<Operator>>,
            impl Into<Expression>,
        ),
        start: usize,
        end: usize,
    ) -> Expression {
        let binary = Binary::new(input.0, input.1.into(), input.2);

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Binary(Box::new(binary)),
        }
    }

    pub fn path(
        &mut self,
        input: (
            &dyn Fn(&mut ExpressionBuilder) -> Expression,
            Vec<impl Into<String>>,
        ),
    ) -> Expression {
        let start = self.pos;

        let head = (input.0)(self);

        let mut tail = vec![];

        for item in input.1 {
            self.consume(".");
            let item = item.into();
            let (start, end) = self.consume(&item);
            tail.push(Spanned::new(Span::from((start, end)), item));
        }

        let end = self.pos;

        ExpressionBuilder::spanned_path((head, tail), start, end)
    }

    pub fn spanned_path(
        input: (impl Into<Expression>, Vec<Spanned<String>>),
        start: usize,
        end: usize,
    ) -> Expression {
        let path = Path::new(input.0.into(), input.1);

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Path(Box::new(path)),
        }
    }

    pub fn call(
        &mut self,
        input: (
            &(dyn Fn(&mut ExpressionBuilder) -> Expression),
            Vec<&dyn Fn(&mut ExpressionBuilder) -> Expression>,
        ),
    ) -> Expression {
        let start = self.pos;

        let name = (&input.0)(self);

        let mut args = vec![];

        for item in input.1 {
            self.consume(" ");
            args.push(item(self));
        }

        let end = self.pos;

        ExpressionBuilder::spanned_call((name, args), start, end)
    }

    pub fn spanned_call(input: impl Into<Call>, start: usize, end: usize) -> Expression {
        let call = input.into();

        Expression {
            span: Span::from((start, end)),
            expr: RawExpression::Call(Box::new(call)),
        }
    }

    pub fn var(&mut self, input: impl Into<String>) -> Expression {
        let input = input.into();
        let (start, _) = self.consume("$");
        let (_, end) = self.consume(&input);

        ExpressionBuilder::spanned_var(input, start, end)
    }

    pub fn spanned_var(input: impl Into<String>, start: usize, end: usize) -> Expression {
        let input = input.into();

        let expr = match &input[..] {
            "it" => RawExpression::VariableReference(Variable::It),
            _ => RawExpression::VariableReference(Variable::Other(input)),
        };

        Expression {
            span: Span::from((start, end)),
            expr,
        }
    }

    pub fn pipeline(
        &mut self,
        input: Vec<&dyn Fn(&mut ExpressionBuilder) -> Expression>,
    ) -> Pipeline {
        let start = self.pos;

        let mut exprs = vec![];
        let mut input = input.into_iter();

        let next = input.next().unwrap();
        exprs.push(next(self));

        for item in input {
            self.consume(" | ");
            exprs.push(item(self));
        }

        let end = self.pos;

        ExpressionBuilder::spanned_pipeline(exprs, start, end)
    }

    pub fn spanned_pipeline(input: Vec<Expression>, start: usize, end: usize) -> Pipeline {
        Pipeline {
            span: Span::from((start, end)),
            commands: input,
        }
    }

    pub fn sp(&mut self) {
        self.consume(" ");
    }

    pub fn ws(&mut self, input: &str) {
        self.consume(input);
    }

    fn consume(&mut self, input: &str) -> (usize, usize) {
        let start = self.pos;
        self.pos += input.len();
        (start, self.pos)
    }
}

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
    // crate fn leaf(leaf: impl Into<Leaf>) -> Expression {
    //     Expression::Leaf(leaf.into())
    // }

    // crate fn flag(flag: impl Into<Flag>) -> Expression {
    //     Expression::Flag(flag.into())
    // }

    // crate fn call(head: Expression, tail: Vec<Expression>) -> Expression {
    //     if tail.len() == 0 {
    //         Expression::Call(Box::new(ParsedCommand::new(head.into(), None)))
    //     } else {
    //         Expression::Call(Box::new(ParsedCommand::new(head.into(), Some(tail))))
    //     }
    // }

    // crate fn binary(
    //     left: impl Into<Expression>,
    //     operator: impl Into<Operator>,
    //     right: impl Into<Expression>,
    // ) -> Expression {
    //     Expression::Binary(Box::new(Binary {
    //         left: left.into(),
    //         operator: operator.into(),
    //         right: right.into(),
    //     }))
    // }

    // crate fn block(expr: impl Into<Expression>) -> Expression {
    //     Expression::Block(Box::new(Block::new(expr.into())))
    // }

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

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, new)]
pub struct Bare {
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

#[derive(new, Debug, Eq, PartialEq)]
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
