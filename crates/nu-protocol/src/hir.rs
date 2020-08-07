use std::cmp::{Ord, Ordering, PartialOrd};
use std::convert::From;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{hir, Primitive, UntaggedValue};
use crate::{PathMember, ShellTypeName};
use derive_new::new;

use nu_errors::ParseError;
use nu_source::{
    b, DebugDocBuilder, HasSpan, PrettyDebug, PrettyDebugRefineKind, PrettyDebugWithSource,
};
use nu_source::{IntoSpanned, Span, Spanned, SpannedItem, Tag};

use bigdecimal::BigDecimal;
use indexmap::IndexMap;
use log::trace;
use num_bigint::{BigInt, ToBigInt};
use num_traits::identities::Zero;
use num_traits::{FromPrimitive, ToPrimitive};

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct InternalCommand {
    pub name: String,
    pub name_span: Span,
    pub args: crate::hir::Call,
}

impl InternalCommand {
    pub fn new(name: String, name_span: Span, full_span: Span) -> InternalCommand {
        InternalCommand {
            name,
            name_span,
            args: crate::hir::Call::new(
                Box::new(SpannedExpression::new(
                    Expression::Command(name_span),
                    name_span,
                )),
                full_span,
            ),
        }
    }

    pub fn expand_it_usage(&mut self) {
        if let Some(positionals) = &mut self.args.positional {
            for arg in positionals {
                arg.expand_it_usage();
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct ClassifiedBlock {
    pub block: Block,
    // this is not a Result to make it crystal clear that these shapes
    // aren't intended to be used directly with `?`
    pub failed: Option<ParseError>,
}

impl ClassifiedBlock {
    pub fn new(block: Block, failed: Option<ParseError>) -> ClassifiedBlock {
        ClassifiedBlock { block, failed }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct ClassifiedPipeline {
    pub commands: Commands,
}

impl ClassifiedPipeline {
    pub fn new(commands: Commands) -> ClassifiedPipeline {
        ClassifiedPipeline { commands }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum ClassifiedCommand {
    Expr(Box<SpannedExpression>),
    #[allow(unused)]
    Dynamic(crate::hir::Call),
    Internal(InternalCommand),
    Error(ParseError),
}

impl ClassifiedCommand {
    pub fn has_it_iteration(&self) -> bool {
        match self {
            ClassifiedCommand::Internal(command) => {
                let mut result = command.args.head.has_shallow_it_usage();

                if let Some(positionals) = &command.args.positional {
                    for arg in positionals {
                        result = result || arg.has_shallow_it_usage();
                    }
                }

                if let Some(named) = &command.args.named {
                    for arg in named.iter() {
                        if let NamedValue::Value(_, value) = arg.1 {
                            result = result || value.has_shallow_it_usage();
                        }
                    }
                }

                result
            }
            ClassifiedCommand::Expr(expr) => expr.has_shallow_it_usage(),
            _ => false,
        }
    }

    pub fn expand_it_usage(&mut self) {
        match self {
            ClassifiedCommand::Internal(command) => command.expand_it_usage(),
            ClassifiedCommand::Expr(expr) => expr.expand_it_usage(),
            _ => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct Commands {
    pub list: Vec<ClassifiedCommand>,
    pub span: Span,
}

impl Commands {
    pub fn new(span: Span) -> Commands {
        Commands { list: vec![], span }
    }

    pub fn push(&mut self, command: ClassifiedCommand) {
        self.list.push(command);
    }

    /// Convert all shallow uses of $it to `each { use of $it }`, converting each to a per-row command
    pub fn expand_it_usage(&mut self) {
        for idx in 0..self.list.len() {
            self.list[idx].expand_it_usage();
        }
        for idx in 1..self.list.len() {
            if self.list[idx].has_it_iteration() {
                self.list[idx] = ClassifiedCommand::Internal(InternalCommand {
                    name: "each".to_string(),
                    name_span: self.span,
                    args: hir::Call {
                        head: Box::new(SpannedExpression {
                            expr: Expression::Synthetic(Synthetic::String(
                                "expanded-each".to_string(),
                            )),
                            span: self.span,
                        }),
                        named: None,
                        span: self.span,
                        positional: Some(vec![SpannedExpression {
                            expr: Expression::Block(Block {
                                block: vec![Commands {
                                    list: vec![self.list[idx].clone()],
                                    span: self.span,
                                }],
                                span: self.span,
                            }),
                            span: self.span,
                        }]),
                        external_redirection: ExternalRedirection::Stdout, // FIXME
                    },
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct Block {
    pub block: Vec<Commands>,
    pub span: Span,
}

impl Block {
    pub fn new(span: Span) -> Block {
        Block {
            block: vec![],
            span,
        }
    }

    pub fn push(&mut self, commands: Commands) {
        self.block.push(commands);
    }

    /// Convert all shallow uses of $it to `each { use of $it }`, converting each to a per-row command
    pub fn expand_it_usage(&mut self) {
        for commands in &mut self.block {
            commands.expand_it_usage();
        }
    }

    pub fn set_redirect(&mut self, external_redirection: ExternalRedirection) {
        if let Some(pipeline) = self.block.last_mut() {
            if let Some(command) = pipeline.list.last_mut() {
                if let ClassifiedCommand::Internal(internal) = command {
                    internal.args.external_redirection = external_redirection;
                }
            }
        }
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Deserialize, Serialize)]
pub struct ExternalStringCommand {
    pub name: Spanned<String>,
    pub args: Vec<Spanned<String>>,
}

impl ExternalArgs {
    pub fn iter(&self) -> impl Iterator<Item = &SpannedExpression> {
        self.list.iter()
    }
}

impl std::ops::Deref for ExternalArgs {
    type Target = [SpannedExpression];

    fn deref(&self) -> &[SpannedExpression] {
        &self.list
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct ExternalArgs {
    pub list: Vec<SpannedExpression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct ExternalCommand {
    pub name: String,

    pub name_tag: Tag,
    pub args: ExternalArgs,
}

impl ExternalCommand {
    pub fn has_it_argument(&self) -> bool {
        self.args.iter().any(|arg| match arg {
            SpannedExpression {
                expr: Expression::Path(path),
                ..
            } => {
                let Path { head, .. } = &**path;
                matches!(head, SpannedExpression{expr: Expression::Variable(Variable::It(_)), ..})
            }
            _ => false,
        })
    }
}

impl HasSpan for ExternalCommand {
    fn span(&self) -> Span {
        self.name_tag.span.until(self.args.span)
    }
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
    Nanosecond,
    Microsecond,
    Millisecond,
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

impl ToBigInt for Number {
    fn to_bigint(&self) -> Option<BigInt> {
        match self {
            Number::Int(int) => Some(int.clone()),
            // The BigDecimal to BigInt conversion always return Some().
            // FIXME: This conversion might not be want we want, it just remove the scale.
            Number::Decimal(decimal) => decimal.to_bigint(),
        }
    }
}

impl PrettyDebug for Unit {
    fn pretty(&self) -> DebugDocBuilder {
        b::keyword(self.as_str())
    }
}

pub fn convert_number_to_u64(number: &Number) -> u64 {
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
            Unit::Nanosecond => "ns",
            Unit::Microsecond => "us",
            Unit::Millisecond => "ms",
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
            Unit::Byte => filesize(convert_number_to_u64(&size)),
            Unit::Kilobyte => filesize(convert_number_to_u64(&size) * 1024),
            Unit::Megabyte => filesize(convert_number_to_u64(&size) * 1024 * 1024),
            Unit::Gigabyte => filesize(convert_number_to_u64(&size) * 1024 * 1024 * 1024),
            Unit::Terabyte => filesize(convert_number_to_u64(&size) * 1024 * 1024 * 1024 * 1024),
            Unit::Petabyte => {
                filesize(convert_number_to_u64(&size) * 1024 * 1024 * 1024 * 1024 * 1024)
            }
            Unit::Nanosecond => duration(size.to_bigint().expect("Conversion should never fail.")),
            Unit::Microsecond => {
                duration(size.to_bigint().expect("Conversion should never fail.") * 1000)
            }
            Unit::Millisecond => {
                duration(size.to_bigint().expect("Conversion should never fail.") * 1000 * 1000)
            }
            Unit::Second => duration(
                size.to_bigint().expect("Conversion should never fail.") * 1000 * 1000 * 1000,
            ),
            Unit::Minute => duration(
                size.to_bigint().expect("Conversion should never fail.") * 60 * 1000 * 1000 * 1000,
            ),
            Unit::Hour => duration(
                size.to_bigint().expect("Conversion should never fail.")
                    * 60
                    * 60
                    * 1000
                    * 1000
                    * 1000,
            ),
            Unit::Day => duration(
                size.to_bigint().expect("Conversion should never fail.")
                    * 24
                    * 60
                    * 60
                    * 1000
                    * 1000
                    * 1000,
            ),
            Unit::Week => duration(
                size.to_bigint().expect("Conversion should never fail.")
                    * 7
                    * 24
                    * 60
                    * 60
                    * 1000
                    * 1000
                    * 1000,
            ),
            // FIXME: Number of days per month should not always be 30.
            Unit::Month => duration(
                size.to_bigint().expect("Conversion should never fail.")
                    * 30
                    * 24
                    * 60
                    * 60
                    * 1000
                    * 1000
                    * 1000,
            ),
            // FIXME: Number of days per year should not be 365.
            Unit::Year => duration(
                size.to_bigint().expect("Conversion should never fail.")
                    * 365
                    * 24
                    * 60
                    * 60
                    * 1000
                    * 1000
                    * 1000,
            ),
        }
    }
}

pub fn filesize(size_in_bytes: u64) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::Filesize(size_in_bytes))
}

pub fn duration(nanos: BigInt) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::Duration(nanos))
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

    pub fn precedence(&self) -> usize {
        match self.expr {
            Expression::Literal(Literal::Operator(operator)) => {
                // Higher precedence binds tighter

                match operator {
                    Operator::Multiply | Operator::Divide => 100,
                    Operator::Plus | Operator::Minus => 90,
                    Operator::NotContains
                    | Operator::Contains
                    | Operator::LessThan
                    | Operator::LessThanOrEqual
                    | Operator::GreaterThan
                    | Operator::GreaterThanOrEqual
                    | Operator::Equal
                    | Operator::NotEqual
                    | Operator::In
                    | Operator::NotIn => 80,
                    Operator::And => 50,
                    Operator::Or => 40, // TODO: should we have And and Or be different precedence?
                }
            }
            _ => 0,
        }
    }

    pub fn has_shallow_it_usage(&self) -> bool {
        match &self.expr {
            Expression::Binary(binary) => {
                binary.left.has_shallow_it_usage() || binary.right.has_shallow_it_usage()
            }
            Expression::Variable(Variable::It(_)) => true,
            Expression::Path(path) => path.head.has_shallow_it_usage(),
            Expression::List(list) => {
                for l in list {
                    if l.has_shallow_it_usage() {
                        return true;
                    }
                }
                false
            }
            Expression::Invocation(block) => {
                for commands in block.block.iter() {
                    for command in commands.list.iter() {
                        if command.has_it_iteration() {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub fn expand_it_usage(&mut self) {
        match self {
            SpannedExpression {
                expr: Expression::Block(block),
                ..
            } => {
                block.expand_it_usage();
            }
            SpannedExpression {
                expr: Expression::Invocation(block),
                ..
            } => {
                block.expand_it_usage();
            }
            SpannedExpression {
                expr: Expression::List(list),
                ..
            } => {
                for item in list.iter_mut() {
                    item.expand_it_usage();
                }
            }
            SpannedExpression {
                expr: Expression::Path(path),
                ..
            } => {
                if let SpannedExpression {
                    expr: Expression::Invocation(block),
                    ..
                } = &mut path.head
                {
                    block.expand_it_usage();
                }
            }
            _ => {}
        }
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
                Expression::Invocation(_) => b::opaque("invocation"),
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
            Expression::Invocation(_) => b::opaque("invocation"),
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
pub enum Operator {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
    Contains,
    NotContains,
    Plus,
    Minus,
    Multiply,
    Divide,
    In,
    NotIn,
    And,
    Or,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Deserialize, Serialize, new)]
pub struct Binary {
    pub left: SpannedExpression,
    pub op: SpannedExpression,
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
    Operator(Operator),
    String(String),
    GlobPattern(String),
    ColumnPath(Vec<Member>),
    Bare(String),
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
            Literal::Bare(_) => "string",
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
                Literal::Bare(bare) => b::delimit("b\"", b::primitive(bare), "\""),
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
            Literal::Bare(bare) => b::typed("bare", b::primitive(bare)),
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
    Block(hir::Block),
    List(Vec<SpannedExpression>),
    Path(Box<Path>),

    FilePath(PathBuf),
    ExternalCommand(ExternalStringCommand),
    Command(Span),
    Invocation(hir::Block),

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
            Expression::Invocation(..) => "command invocation",
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

    pub fn operator(operator: Operator) -> Expression {
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

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum ExternalRedirection {
    None,
    Stdout,
    Stderr,
    StdoutAndStderr,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct Call {
    pub head: Box<SpannedExpression>,
    pub positional: Option<Vec<SpannedExpression>>,
    pub named: Option<NamedArguments>,
    pub span: Span,
    pub external_redirection: ExternalRedirection,
}

impl Call {
    pub fn switch_preset(&self, switch: &str) -> bool {
        self.named
            .as_ref()
            .map(|n| n.switch_present(switch))
            .unwrap_or(false)
    }

    pub fn set_initial_flags(&mut self, signature: &crate::Signature) {
        for (named, value) in signature.named.iter() {
            if self.named.is_none() {
                self.named = Some(NamedArguments::new());
            }

            if let Some(ref mut args) = self.named {
                match value.0 {
                    crate::NamedType::Switch(_) => args.insert_switch(named, None),
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
            external_redirection: ExternalRedirection::Stdout,
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
    Operator,
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedArguments {
    pub named: IndexMap<String, NamedValue>,
}

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for NamedArguments {
    /// Create the hash function to allow the Hash trait for dictionaries
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut entries = self.named.clone();
        entries.sort_keys();
        entries.keys().collect::<Vec<&String>>().hash(state);
        entries.values().collect::<Vec<&NamedValue>>().hash(state);
    }
}

impl PartialOrd for NamedArguments {
    /// Compare two dictionaries for sort ordering
    fn partial_cmp(&self, other: &NamedArguments) -> Option<Ordering> {
        let this: Vec<&String> = self.named.keys().collect();
        let that: Vec<&String> = other.named.keys().collect();

        if this != that {
            return this.partial_cmp(&that);
        }

        let this: Vec<&NamedValue> = self.named.values().collect();
        let that: Vec<&NamedValue> = self.named.values().collect();

        this.partial_cmp(&that)
    }
}

impl Ord for NamedArguments {
    /// Compare two dictionaries for ordering
    fn cmp(&self, other: &NamedArguments) -> Ordering {
        let this: Vec<&String> = self.named.keys().collect();
        let that: Vec<&String> = other.named.keys().collect();

        if this != that {
            return this.cmp(&that);
        }

        let this: Vec<&NamedValue> = self.named.values().collect();
        let that: Vec<&NamedValue> = self.named.values().collect();

        this.cmp(&that)
    }
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
            .map(|t| matches!(t, NamedValue::PresentSwitch(_)))
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
