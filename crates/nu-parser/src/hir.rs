use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use derive_new::new;
use nu_protocol::{PathMember, ShellTypeName};
use nu_protocol::{Primitive, UntaggedValue};
use num_traits::ToPrimitive;

use nu_source::{
    b, DebugDocBuilder, HasSpan, PrettyDebug, PrettyDebugRefineKind, PrettyDebugWithSource,
};
use nu_source::{IntoSpanned, Span, Spanned, SpannedItem};

use bigdecimal::BigDecimal;
use indexmap::IndexMap;
use log::trace;
use num_bigint::BigInt;
use num_traits::identities::Zero;
use num_traits::FromPrimitive;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Deserialize, Serialize)]
pub struct ExternalCommand {
    pub name: Spanned<String>,
    pub args: Vec<Spanned<String>>,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Copy, Deserialize, Serialize)]
pub enum Unit {
    // Filesize units
    Byte,
    Kilobyte,
    Megabyte,
    Gigabyte,
    Terabyte,
    Petabyte,

    // Duration units
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Deserialize, Serialize)]
pub enum Member {
    String(/* outer */ Span, /* inner */ Span),
    Int(BigInt, Span),
    Bare(Spanned<String>),
}

impl Member {
    // pub fn int(span: Span, source: &Text) -> Member {
    //     if let Ok(big_int) = BigInt::from_str(span.slice(source)) {
    //         Member::Int(big_int, span)
    //     } else {
    //         unreachable!("Internal error: could not convert text to BigInt as expected")
    //     }
    // }

    pub fn to_path_member(&self) -> PathMember {
        match self {
            //Member::String(outer, inner) => PathMember::string(inner.slice(source), *outer),
            Member::Int(int, span) => PathMember::int(int.clone(), *span),
            Member::Bare(spanned_string) => {
                PathMember::string(spanned_string.item.clone(), spanned_string.span)
            }
            _ => unimplemented!("Need to finish to_path_member"),
        }
    }
}

impl PrettyDebugWithSource for Member {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self {
            Member::String(outer, _) => b::value(outer.slice(source)),
            Member::Int(int, _) => b::value(format!("{}", int)),
            Member::Bare(span) => b::value(span.span.slice(source)),
        }
    }
}

impl HasSpan for Member {
    fn span(&self) -> Span {
        match self {
            Member::String(outer, ..) => *outer,
            Member::Int(_, int) => *int,
            Member::Bare(name) => name.span,
        }
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Deserialize, Serialize)]
pub enum Number {
    Int(BigInt),
    Decimal(BigDecimal),
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

impl PrettyDebug for Unit {
    fn pretty(&self) -> DebugDocBuilder {
        b::keyword(self.as_str())
    }
}

fn convert_number_to_u64(number: &Number) -> u64 {
    match number {
        Number::Int(big_int) => {
            if let Some(x) = big_int.to_u64() {
                x
            } else {
                unreachable!("Internal error: convert_number_to_u64 given incompatible number")
            }
        }
        Number::Decimal(big_decimal) => {
            if let Some(x) = big_decimal.to_u64() {
                x
            } else {
                unreachable!("Internal error: convert_number_to_u64 given incompatible number")
            }
        }
    }
}

impl Unit {
    pub fn as_str(self) -> &'static str {
        match self {
            Unit::Byte => "B",
            Unit::Kilobyte => "KB",
            Unit::Megabyte => "MB",
            Unit::Gigabyte => "GB",
            Unit::Terabyte => "TB",
            Unit::Petabyte => "PB",
            Unit::Second => "s",
            Unit::Minute => "m",
            Unit::Hour => "h",
            Unit::Day => "d",
            Unit::Week => "w",
            Unit::Month => "M",
            Unit::Year => "y",
        }
    }

    pub fn compute(self, size: &Number) -> UntaggedValue {
        let size = size.clone();

        match self {
            Unit::Byte => number(size),
            Unit::Kilobyte => number(size * 1024),
            Unit::Megabyte => number(size * 1024 * 1024),
            Unit::Gigabyte => number(size * 1024 * 1024 * 1024),
            Unit::Terabyte => number(size * 1024 * 1024 * 1024 * 1024),
            Unit::Petabyte => number(size * 1024 * 1024 * 1024 * 1024 * 1024),
            Unit::Second => duration(convert_number_to_u64(&size)),
            Unit::Minute => duration(60 * convert_number_to_u64(&size)),
            Unit::Hour => duration(60 * 60 * convert_number_to_u64(&size)),
            Unit::Day => duration(24 * 60 * 60 * convert_number_to_u64(&size)),
            Unit::Week => duration(7 * 24 * 60 * 60 * convert_number_to_u64(&size)),
            Unit::Month => duration(30 * 24 * 60 * 60 * convert_number_to_u64(&size)),
            Unit::Year => duration(365 * 24 * 60 * 60 * convert_number_to_u64(&size)),
        }
    }
}

fn number(number: impl Into<Number>) -> UntaggedValue {
    let number = number.into();

    match number {
        Number::Int(int) => UntaggedValue::Primitive(Primitive::Int(int)),
        Number::Decimal(decimal) => UntaggedValue::Primitive(Primitive::Decimal(decimal)),
    }
}

pub fn duration(secs: u64) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::Duration(secs))
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Deserialize, Serialize)]
pub struct SpannedExpression {
    pub expr: Expression,
    pub span: Span,
}

impl SpannedExpression {
    pub fn new(expr: Expression, span: Span) -> SpannedExpression {
        SpannedExpression { expr, span }
    }
}

impl std::ops::Deref for SpannedExpression {
    type Target = Expression;

    fn deref(&self) -> &Expression {
        &self.expr
    }
}

impl HasSpan for SpannedExpression {
    fn span(&self) -> Span {
        self.span
    }
}

impl ShellTypeName for SpannedExpression {
    fn type_name(&self) -> &'static str {
        self.expr.type_name()
    }
}

impl PrettyDebugWithSource for SpannedExpression {
    fn refined_pretty_debug(&self, refine: PrettyDebugRefineKind, source: &str) -> DebugDocBuilder {
        match refine {
            PrettyDebugRefineKind::ContextFree => self.refined_pretty_debug(refine, source),
            PrettyDebugRefineKind::WithContext => match &self.expr {
                Expression::Literal(literal) => literal
                    .clone()
                    .into_spanned(self.span)
                    .refined_pretty_debug(refine, source),
                Expression::ExternalWord => {
                    b::delimit("e\"", b::primitive(self.span.slice(source)), "\"").group()
                }
                Expression::Synthetic(s) => match s {
                    Synthetic::String(_) => {
                        b::delimit("s\"", b::primitive(self.span.slice(source)), "\"").group()
                    }
                },
                Expression::Variable(Variable::Other(_, _)) => b::keyword(self.span.slice(source)),
                Expression::Variable(Variable::It(_)) => b::keyword("$it"),
                Expression::Binary(binary) => binary.pretty_debug(source),
                Expression::Range(range) => range.pretty_debug(source),
                Expression::Block(_) => b::opaque("block"),
                Expression::Garbage => b::opaque("garbage"),
                Expression::List(list) => b::delimit(
                    "[",
                    b::intersperse(
                        list.iter()
                            .map(|item| item.refined_pretty_debug(refine, source)),
                        b::space(),
                    ),
                    "]",
                ),
                Expression::Path(path) => path.pretty_debug(source),
                Expression::FilePath(path) => b::typed("path", b::primitive(path.display())),
                Expression::ExternalCommand(external) => {
                    b::keyword("^") + b::keyword(external.name.span.slice(source))
                }
                Expression::Command(command) => b::keyword(command.slice(source)),
                Expression::Boolean(boolean) => match boolean {
                    true => b::primitive("$yes"),
                    false => b::primitive("$no"),
                },
            },
        }
    }

    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match &self.expr {
            Expression::Literal(literal) => {
                literal.clone().into_spanned(self.span).pretty_debug(source)
            }
            Expression::ExternalWord => {
                b::typed("external word", b::primitive(self.span.slice(source)))
            }
            Expression::Synthetic(s) => match s {
                Synthetic::String(s) => b::typed("synthetic", b::primitive(format!("{:?}", s))),
            },
            Expression::Variable(Variable::Other(_, _)) => b::keyword(self.span.slice(source)),
            Expression::Variable(Variable::It(_)) => b::keyword("$it"),
            Expression::Binary(binary) => binary.pretty_debug(source),
            Expression::Range(range) => range.pretty_debug(source),
            Expression::Block(_) => b::opaque("block"),
            Expression::Garbage => b::opaque("garbage"),
            Expression::List(list) => b::delimit(
                "[",
                b::intersperse(
                    list.iter().map(|item| item.pretty_debug(source)),
                    b::space(),
                ),
                "]",
            ),
            Expression::Path(path) => path.pretty_debug(source),
            Expression::FilePath(path) => b::typed("path", b::primitive(path.display())),
            Expression::ExternalCommand(external) => b::typed(
                "command",
                b::keyword("^") + b::primitive(external.name.span.slice(source)),
            ),
            Expression::Command(command) => {
                b::typed("command", b::primitive(command.slice(source)))
            }
            Expression::Boolean(boolean) => match boolean {
                true => b::primitive("$yes"),
                false => b::primitive("$no"),
            },
        }
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Deserialize, Serialize)]
pub enum Variable {
    It(Span),
    Other(String, Span),
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum CompareOperator {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
    Contains,
    NotContains,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Deserialize, Serialize, new)]
pub struct Binary {
    pub left: SpannedExpression,
    pub op: SpannedExpression, //Spanned<CompareOperator>,
    pub right: SpannedExpression,
}

impl PrettyDebugWithSource for Binary {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::delimit(
            "<",
            self.left.pretty_debug(source)
                + b::space()
                + b::keyword(self.op.span.slice(source))
                + b::space()
                + self.right.pretty_debug(source),
            ">",
        )
        .group()
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Deserialize, Serialize)]
pub enum Synthetic {
    String(String),
}

impl ShellTypeName for Synthetic {
    fn type_name(&self) -> &'static str {
        match self {
            Synthetic::String(_) => "string",
        }
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Deserialize, Serialize)]
pub struct Range {
    pub left: SpannedExpression,
    pub dotdot: Span,
    pub right: SpannedExpression,
}

impl PrettyDebugWithSource for Range {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::delimit(
            "<",
            self.left.pretty_debug(source)
                + b::space()
                + b::keyword(self.dotdot.slice(source))
                + b::space()
                + self.right.pretty_debug(source),
            ">",
        )
        .group()
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Deserialize, Serialize)]
pub enum Literal {
    Number(Number),
    Size(Spanned<Number>, Spanned<Unit>),
    Operator(CompareOperator),
    String(String),
    GlobPattern(String),
    ColumnPath(Vec<Member>),
    Bare,
}

impl Literal {
    pub fn into_spanned(self, span: impl Into<Span>) -> SpannedLiteral {
        SpannedLiteral {
            literal: self,
            span: span.into(),
        }
    }
}

//, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize
#[derive(Debug, Clone)]
pub struct SpannedLiteral {
    pub literal: Literal,
    pub span: Span,
}

impl ShellTypeName for Literal {
    fn type_name(&self) -> &'static str {
        match &self {
            Literal::Number(..) => "number",
            Literal::Size(..) => "size",
            Literal::String(..) => "string",
            Literal::ColumnPath(..) => "column path",
            Literal::Bare => "string",
            Literal::GlobPattern(_) => "pattern",
            Literal::Operator(_) => "operator",
        }
    }
}

impl PrettyDebugWithSource for SpannedLiteral {
    fn refined_pretty_debug(&self, refine: PrettyDebugRefineKind, source: &str) -> DebugDocBuilder {
        match refine {
            PrettyDebugRefineKind::ContextFree => self.pretty_debug(source),
            PrettyDebugRefineKind::WithContext => match &self.literal {
                Literal::Number(number) => number.pretty(),
                Literal::Size(number, unit) => (number.pretty() + unit.pretty()).group(),
                Literal::String(string) => b::primitive(format!("{:?}", string)), //string.slice(source))),
                Literal::GlobPattern(pattern) => b::primitive(pattern),
                Literal::ColumnPath(path) => {
                    b::intersperse_with_source(path.iter(), b::space(), source)
                }
                Literal::Bare => b::delimit("b\"", b::primitive(self.span.slice(source)), "\""),
                Literal::Operator(operator) => b::primitive(format!("{:?}", operator)),
            },
        }
    }

    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match &self.literal {
            Literal::Number(number) => number.pretty(),
            Literal::Size(number, unit) => {
                b::typed("size", (number.pretty() + unit.pretty()).group())
            }
            Literal::String(string) => b::typed(
                "string",
                b::primitive(format!("{:?}", string)), //string.slice(source))),
            ),
            Literal::GlobPattern(pattern) => b::typed("pattern", b::primitive(pattern)),
            Literal::ColumnPath(path) => b::typed(
                "column path",
                b::intersperse_with_source(path.iter(), b::space(), source),
            ),
            Literal::Bare => b::typed("bare", b::primitive(self.span.slice(source))),
            Literal::Operator(operator) => {
                b::typed("operator", b::primitive(format!("{:?}", operator)))
            }
        }
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, new, Deserialize, Serialize)]
pub struct Path {
    pub head: SpannedExpression,
    pub tail: Vec<PathMember>,
}

impl PrettyDebugWithSource for Path {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        self.head.pretty_debug(source)
            + b::operator(".")
            + b::intersperse(self.tail.iter().map(|m| m.pretty()), b::operator("."))
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Deserialize, Serialize)]
pub enum Expression {
    Literal(Literal),
    ExternalWord,
    Synthetic(Synthetic),
    Variable(Variable),
    Binary(Box<Binary>),
    Range(Box<Range>),
    Block(Vec<SpannedExpression>),
    List(Vec<SpannedExpression>),
    Path(Box<Path>),

    FilePath(PathBuf),
    ExternalCommand(ExternalCommand),
    Command(Span),

    Boolean(bool),

    // Trying this approach out: if we let parsing always be infallible
    // we can use the same parse and just place bad token markers in the output
    // We can later throw an error if we try to process them further.
    Garbage,
}

impl ShellTypeName for Expression {
    fn type_name(&self) -> &'static str {
        match self {
            Expression::Literal(literal) => literal.type_name(),
            Expression::Synthetic(synthetic) => synthetic.type_name(),
            Expression::Command(..) => "command",
            Expression::ExternalWord => "external word",
            Expression::FilePath(..) => "file path",
            Expression::Variable(..) => "variable",
            Expression::List(..) => "list",
            Expression::Binary(..) => "binary",
            Expression::Range(..) => "range",
            Expression::Block(..) => "block",
            Expression::Path(..) => "variable path",
            Expression::Boolean(..) => "boolean",
            Expression::ExternalCommand(..) => "external",
            Expression::Garbage => "garbage",
        }
    }
}

impl IntoSpanned for Expression {
    type Output = SpannedExpression;

    fn into_spanned(self, span: impl Into<Span>) -> Self::Output {
        SpannedExpression {
            expr: self,
            span: span.into(),
        }
    }
}

impl Expression {
    pub fn integer(i: i64) -> Expression {
        Expression::Literal(Literal::Number(Number::Int(BigInt::from(i))))
    }

    pub fn decimal(f: f64) -> Expression {
        Expression::Literal(Literal::Number(Number::Decimal(BigDecimal::from(f))))
    }

    pub fn string(s: String) -> Expression {
        Expression::Literal(Literal::String(s))
    }

    pub fn operator(operator: CompareOperator) -> Expression {
        Expression::Literal(Literal::Operator(operator))
    }

    pub fn range(left: SpannedExpression, dotdot: Span, right: SpannedExpression) -> Expression {
        Expression::Range(Box::new(Range {
            left,
            dotdot,
            right,
        }))
    }

    pub fn pattern(p: String) -> Expression {
        Expression::Literal(Literal::GlobPattern(p))
    }

    pub fn file_path(file_path: PathBuf) -> Expression {
        Expression::FilePath(file_path)
    }

    pub fn simple_column_path(members: Vec<Member>) -> Expression {
        Expression::Literal(Literal::ColumnPath(members))
    }

    pub fn path(head: SpannedExpression, tail: Vec<impl Into<PathMember>>) -> Expression {
        let tail = tail.into_iter().map(|t| t.into()).collect();
        Expression::Path(Box::new(Path::new(head, tail)))
    }

    pub fn unit(i: Spanned<i64>, unit: Spanned<Unit>) -> Expression {
        Expression::Literal(Literal::Size(
            Number::Int(BigInt::from(i.item)).spanned(i.span),
            unit,
        ))
    }

    pub fn variable(v: String, span: Span) -> Expression {
        if v == "$it" {
            Expression::Variable(Variable::It(span))
        } else {
            Expression::Variable(Variable::Other(v, span))
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum NamedValue {
    AbsentSwitch,
    PresentSwitch(Span),
    AbsentValue,
    Value(Span, SpannedExpression),
}

impl PrettyDebugWithSource for NamedValue {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self {
            NamedValue::AbsentSwitch => b::typed("switch", b::description("absent")),
            NamedValue::PresentSwitch(_) => b::typed("switch", b::description("present")),
            NamedValue::AbsentValue => b::description("absent"),
            NamedValue::Value(_, value) => value.pretty_debug(source),
        }
    }

    fn refined_pretty_debug(&self, refine: PrettyDebugRefineKind, source: &str) -> DebugDocBuilder {
        match refine {
            PrettyDebugRefineKind::ContextFree => self.pretty_debug(source),
            PrettyDebugRefineKind::WithContext => match self {
                NamedValue::AbsentSwitch => b::value("absent"),
                NamedValue::PresentSwitch(_) => b::value("present"),
                NamedValue::AbsentValue => b::value("absent"),
                NamedValue::Value(_, value) => value.refined_pretty_debug(refine, source),
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Call {
    pub head: Box<SpannedExpression>,
    pub positional: Option<Vec<SpannedExpression>>,
    pub named: Option<NamedArguments>,
    pub span: Span,
}

impl Call {
    pub fn switch_preset(&self, switch: &str) -> bool {
        self.named
            .as_ref()
            .map(|n| n.switch_present(switch))
            .unwrap_or(false)
    }

    pub fn set_initial_flags(&mut self, signature: &nu_protocol::Signature) {
        for (named, value) in signature.named.iter() {
            if self.named.is_none() {
                self.named = Some(NamedArguments::new());
            }

            if let Some(ref mut args) = self.named {
                match value.0 {
                    nu_protocol::NamedType::Switch(_) => args.insert_switch(named, None),
                    _ => args.insert_optional(named, Span::new(0, 0), None),
                }
            }
        }
    }
}

impl PrettyDebugWithSource for Call {
    fn refined_pretty_debug(&self, refine: PrettyDebugRefineKind, source: &str) -> DebugDocBuilder {
        match refine {
            PrettyDebugRefineKind::ContextFree => self.pretty_debug(source),
            PrettyDebugRefineKind::WithContext => {
                self.head
                    .refined_pretty_debug(PrettyDebugRefineKind::WithContext, source)
                    + b::preceded_option(
                        Some(b::space()),
                        self.positional.as_ref().map(|pos| {
                            b::intersperse(
                                pos.iter().map(|expr| {
                                    expr.refined_pretty_debug(
                                        PrettyDebugRefineKind::WithContext,
                                        source,
                                    )
                                }),
                                b::space(),
                            )
                        }),
                    )
                    + b::preceded_option(
                        Some(b::space()),
                        self.named.as_ref().map(|named| {
                            named.refined_pretty_debug(PrettyDebugRefineKind::WithContext, source)
                        }),
                    )
            }
        }
    }

    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::typed(
            "call",
            self.refined_pretty_debug(PrettyDebugRefineKind::WithContext, source),
        )
    }
}

impl Call {
    pub fn new(head: Box<SpannedExpression>, span: Span) -> Call {
        Call {
            head,
            positional: None,
            named: None,
            span,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Delimiter {
    Paren,
    Brace,
    Square,
}

#[derive(Debug, Copy, Clone)]
pub enum FlatShape {
    OpenDelimiter(Delimiter),
    CloseDelimiter(Delimiter),
    Type,
    Identifier,
    ItVariable,
    Variable,
    CompareOperator,
    Dot,
    DotDot,
    InternalCommand,
    ExternalCommand,
    ExternalWord,
    BareMember,
    StringMember,
    String,
    Path,
    Word,
    Keyword,
    Pipe,
    GlobPattern,
    Flag,
    ShorthandFlag,
    Int,
    Decimal,
    Garbage,
    Whitespace,
    Separator,
    Comment,
    Size { number: Span, unit: Span },
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct NamedArguments {
    pub named: IndexMap<String, NamedValue>,
}

impl NamedArguments {
    pub fn new() -> NamedArguments {
        Default::default()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &NamedValue)> {
        self.named.iter()
    }

    pub fn get(&self, name: &str) -> Option<&NamedValue> {
        self.named.get(name)
    }

    pub fn is_empty(&self) -> bool {
        self.named.is_empty()
    }
}

impl NamedArguments {
    pub fn insert_switch(&mut self, name: impl Into<String>, switch: Option<Flag>) {
        let name = name.into();
        trace!("Inserting switch -- {} = {:?}", name, switch);

        match switch {
            None => self.named.insert(name, NamedValue::AbsentSwitch),
            Some(flag) => self
                .named
                .insert(name, NamedValue::PresentSwitch(flag.name)),
        };
    }

    pub fn insert_optional(
        &mut self,
        name: impl Into<String>,
        flag_span: Span,
        expr: Option<SpannedExpression>,
    ) {
        match expr {
            None => self.named.insert(name.into(), NamedValue::AbsentValue),
            Some(expr) => self
                .named
                .insert(name.into(), NamedValue::Value(flag_span, expr)),
        };
    }

    pub fn insert_mandatory(
        &mut self,
        name: impl Into<String>,
        flag_span: Span,
        expr: SpannedExpression,
    ) {
        self.named
            .insert(name.into(), NamedValue::Value(flag_span, expr));
    }

    pub fn switch_present(&self, switch: &str) -> bool {
        self.named
            .get(switch)
            .map(|t| match t {
                NamedValue::PresentSwitch(_) => true,
                _ => false,
            })
            .unwrap_or(false)
    }
}

impl PrettyDebugWithSource for NamedArguments {
    fn refined_pretty_debug(&self, refine: PrettyDebugRefineKind, source: &str) -> DebugDocBuilder {
        match refine {
            PrettyDebugRefineKind::ContextFree => self.pretty_debug(source),
            PrettyDebugRefineKind::WithContext => b::intersperse(
                self.named.iter().map(|(key, value)| {
                    b::key(key)
                        + b::equals()
                        + value.refined_pretty_debug(PrettyDebugRefineKind::WithContext, source)
                }),
                b::space(),
            ),
        }
    }

    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::delimit(
            "(",
            self.refined_pretty_debug(PrettyDebugRefineKind::WithContext, source),
            ")",
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FlagKind {
    Shorthand,
    Longhand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, new)]
pub struct Flag {
    pub(crate) kind: FlagKind,
    pub(crate) name: Span,
}

impl PrettyDebugWithSource for Flag {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        let prefix = match self.kind {
            FlagKind::Longhand => b::description("--"),
            FlagKind::Shorthand => b::description("-"),
        };

        prefix + b::description(self.name.slice(source))
    }
}

impl Flag {
    pub fn color(&self, span: impl Into<Span>) -> Spanned<FlatShape> {
        match self.kind {
            FlagKind::Longhand => FlatShape::Flag.spanned(span.into()),
            FlagKind::Shorthand => FlatShape::ShorthandFlag.spanned(span.into()),
        }
    }
}
