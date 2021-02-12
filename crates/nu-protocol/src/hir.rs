use std::cmp::{Ord, Ordering, PartialOrd};
use std::convert::From;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::Signature;
use crate::{hir, Dictionary, PositionalType, Primitive, SyntaxShape, UntaggedValue};
use crate::{PathMember, ShellTypeName};
use derive_new::new;

use nu_errors::ParseError;
use nu_source::{
    DbgDocBldr, DebugDocBuilder, HasSpan, PrettyDebug, PrettyDebugRefineKind, PrettyDebugWithSource,
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
                Box::new(SpannedExpression::new(Expression::Command, name_span)),
                full_span,
            ),
        }
    }

    pub fn has_it_usage(&self) -> bool {
        self.args.has_it_usage()
    }

    pub fn get_free_variables(&self, known_variables: &mut Vec<String>) -> Vec<String> {
        self.args.get_free_variables(known_variables)
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
    pub commands: Pipeline,
}

impl ClassifiedPipeline {
    pub fn new(commands: Pipeline) -> ClassifiedPipeline {
        ClassifiedPipeline { commands }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum ClassifiedCommand {
    Expr(Box<SpannedExpression>),
    Dynamic(crate::hir::Call),
    Internal(InternalCommand),
    Error(ParseError),
}

impl ClassifiedCommand {
    fn has_it_usage(&self) -> bool {
        match self {
            ClassifiedCommand::Expr(expr) => expr.has_it_usage(),
            ClassifiedCommand::Dynamic(call) => call.has_it_usage(),
            ClassifiedCommand::Internal(internal) => internal.has_it_usage(),
            ClassifiedCommand::Error(_) => false,
        }
    }

    pub fn get_free_variables(&self, known_variables: &mut Vec<String>) -> Vec<String> {
        match self {
            ClassifiedCommand::Expr(expr) => expr.get_free_variables(known_variables),
            ClassifiedCommand::Dynamic(call) => call.get_free_variables(known_variables),
            ClassifiedCommand::Internal(internal) => internal.get_free_variables(known_variables),
            _ => vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct Pipeline {
    pub list: Vec<ClassifiedCommand>,
    pub span: Span,
}

impl Pipeline {
    pub fn new(span: Span) -> Pipeline {
        Pipeline { list: vec![], span }
    }

    pub fn basic() -> Pipeline {
        Pipeline {
            list: vec![],
            span: Span::unknown(),
        }
    }

    pub fn push(&mut self, command: ClassifiedCommand) {
        self.list.push(command);
    }

    pub fn has_it_usage(&self) -> bool {
        self.list.iter().any(|cc| cc.has_it_usage())
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct Group {
    pub pipelines: Vec<Pipeline>,
    pub span: Span,
}
impl Group {
    pub fn new(pipelines: Vec<Pipeline>, span: Span) -> Group {
        Group { pipelines, span }
    }

    pub fn basic() -> Group {
        Group {
            pipelines: vec![],
            span: Span::unknown(),
        }
    }

    pub fn push(&mut self, pipeline: Pipeline) {
        self.pipelines.push(pipeline);
    }

    pub fn has_it_usage(&self) -> bool {
        self.pipelines.iter().any(|cc| cc.has_it_usage())
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct CapturedBlock {
    pub block: Block,
    pub captured: Dictionary,
}

impl CapturedBlock {
    pub fn new(block: Block, captured: Dictionary) -> Self {
        Self { block, captured }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub params: Signature,
    pub block: Vec<Group>,
    pub definitions: IndexMap<String, Block>,
    pub span: Span,
}

impl Block {
    pub fn new(
        params: Signature,
        block: Vec<Group>,
        definitions: IndexMap<String, Block>,
        span: Span,
    ) -> Block {
        Block {
            params,
            block,
            definitions,
            span,
        }
    }

    pub fn basic() -> Block {
        Block {
            params: Signature::new("<basic>"),
            block: vec![],
            definitions: IndexMap::new(),
            span: Span::unknown(),
        }
    }

    pub fn push(&mut self, group: Group) {
        self.block.push(group);
        self.infer_params();
    }

    pub fn set_redirect(&mut self, external_redirection: ExternalRedirection) {
        if let Some(group) = self.block.last_mut() {
            if let Some(pipeline) = group.pipelines.last_mut() {
                if let Some(ClassifiedCommand::Internal(internal)) = pipeline.list.last_mut() {
                    internal.args.external_redirection = external_redirection;
                }
            }
        }
    }

    pub fn has_it_usage(&self) -> bool {
        self.block.iter().any(|x| x.has_it_usage())
    }

    pub fn infer_params(&mut self) {
        // FIXME: re-enable inference later
        if self.params.positional.is_empty() && self.has_it_usage() {
            self.params.positional = vec![(
                PositionalType::Mandatory("$it".to_string(), SyntaxShape::Any),
                "implied $it".to_string(),
            )];
        }
    }

    pub fn get_free_variables(&self, known_variables: &mut Vec<String>) -> Vec<String> {
        let mut known_variables = known_variables.clone();
        let positional_params: Vec<_> = self
            .params
            .positional
            .iter()
            .map(|(_, name)| name.clone())
            .collect();
        known_variables.extend_from_slice(&positional_params);

        let mut free_variables = vec![];
        for group in &self.block {
            for pipeline in &group.pipelines {
                for elem in &pipeline.list {
                    free_variables
                        .extend_from_slice(&elem.get_free_variables(&mut known_variables));
                }
            }
        }

        free_variables
    }
}

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for Block {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut entries = self.definitions.clone();
        entries.sort_keys();

        // FIXME: this is incomplete
        entries.keys().collect::<Vec<&String>>().hash(state);
    }
}

impl PartialOrd for Block {
    /// Compare two dictionaries for sort ordering
    fn partial_cmp(&self, other: &Block) -> Option<Ordering> {
        let this: Vec<&String> = self.definitions.keys().collect();
        let that: Vec<&String> = other.definitions.keys().collect();

        // FIXME: this is incomplete
        this.partial_cmp(&that)
    }
}

impl Ord for Block {
    /// Compare two dictionaries for ordering
    fn cmp(&self, other: &Block) -> Ordering {
        let this: Vec<&String> = self.definitions.keys().collect();
        let that: Vec<&String> = other.definitions.keys().collect();

        // FIXME: this is incomplete
        this.cmp(&that)
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
    pub fn has_it_usage(&self) -> bool {
        self.args.iter().any(|arg| match arg {
            SpannedExpression {
                expr: Expression::Path(path),
                ..
            } => {
                let Path { head, .. } = &**path;
                matches!(head, SpannedExpression{expr: Expression::Variable(x, ..), ..} if x == "$it")
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
    // Filesize units: metric
    Byte,
    Kilobyte,
    Megabyte,
    Gigabyte,
    Terabyte,
    Petabyte,

    // Filesize units: ISO/IEC 80000
    Kibibyte,
    Mebibyte,
    Gibibyte,

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
            Member::String(outer, _) => DbgDocBldr::value(outer.slice(source)),
            Member::Int(int, _) => DbgDocBldr::value(format!("{}", int)),
            Member::Bare(span) => DbgDocBldr::value(span.span.slice(source)),
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
            Number::Int(int) => DbgDocBldr::primitive(int),
            Number::Decimal(decimal) => DbgDocBldr::primitive(decimal),
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
        DbgDocBldr::keyword(self.as_str())
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
            Unit::Kibibyte => "KiB",
            Unit::Mebibyte => "MiB",
            Unit::Gibibyte => "GiB",
            Unit::Nanosecond => "ns",
            Unit::Microsecond => "us",
            Unit::Millisecond => "ms",
            Unit::Second => "sec",
            Unit::Minute => "min",
            Unit::Hour => "hr",
            Unit::Day => "day",
            Unit::Week => "wk",
            Unit::Month => "mon",
            Unit::Year => "yr",
        }
    }

    pub fn compute(self, size: &Number) -> UntaggedValue {
        let size = size.clone();

        match self {
            Unit::Byte => filesize(convert_number_to_u64(&size)),
            Unit::Kilobyte => filesize(convert_number_to_u64(&size) * 1000),
            Unit::Megabyte => filesize(convert_number_to_u64(&size) * 1000 * 1000),
            Unit::Gigabyte => filesize(convert_number_to_u64(&size) * 1000 * 1000 * 1000),
            Unit::Terabyte => filesize(convert_number_to_u64(&size) * 1000 * 1000 * 1000 * 1000),
            Unit::Petabyte => {
                filesize(convert_number_to_u64(&size) * 1000 * 1000 * 1000 * 1000 * 1000)
            }

            Unit::Kibibyte => filesize(convert_number_to_u64(&size) * 1024),
            Unit::Mebibyte => filesize(convert_number_to_u64(&size) * 1024 * 1024),
            Unit::Gibibyte => filesize(convert_number_to_u64(&size) * 1024 * 1024 * 1024),

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

pub fn filesize(size_in_bytes: impl Into<BigInt>) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::Filesize(size_in_bytes.into()))
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
                    Operator::Pow => 100,
                    Operator::Multiply | Operator::Divide | Operator::Modulo => 95,
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

    pub fn has_it_usage(&self) -> bool {
        self.expr.has_it_usage()
    }

    pub fn get_free_variables(&self, known_variables: &mut Vec<String>) -> Vec<String> {
        self.expr.get_free_variables(known_variables)
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
                    DbgDocBldr::delimit("e\"", DbgDocBldr::primitive(self.span.slice(source)), "\"")
                        .group()
                }
                Expression::Synthetic(s) => match s {
                    Synthetic::String(_) => DbgDocBldr::delimit(
                        "s\"",
                        DbgDocBldr::primitive(self.span.slice(source)),
                        "\"",
                    )
                    .group(),
                },
                Expression::Variable(_, _) => DbgDocBldr::keyword(self.span.slice(source)),
                Expression::Binary(binary) => binary.pretty_debug(source),
                Expression::Range(range) => range.pretty_debug(source),
                Expression::Block(_) => DbgDocBldr::opaque("block"),
                Expression::Invocation(_) => DbgDocBldr::opaque("invocation"),
                Expression::Garbage => DbgDocBldr::opaque("garbage"),
                Expression::List(list) => DbgDocBldr::delimit(
                    "[",
                    DbgDocBldr::intersperse(
                        list.iter()
                            .map(|item| item.refined_pretty_debug(refine, source)),
                        DbgDocBldr::space(),
                    ),
                    "]",
                ),
                Expression::Table(_headers, cells) => DbgDocBldr::delimit(
                    "[",
                    DbgDocBldr::intersperse(
                        cells
                            .iter()
                            .map(|row| {
                                row.iter()
                                    .map(|item| item.refined_pretty_debug(refine, source))
                            })
                            .flatten(),
                        DbgDocBldr::space(),
                    ),
                    "]",
                ),
                Expression::Path(path) => path.pretty_debug(source),
                Expression::FilePath(path) => {
                    DbgDocBldr::typed("path", DbgDocBldr::primitive(path.display()))
                }
                Expression::ExternalCommand(external) => {
                    DbgDocBldr::keyword("^") + DbgDocBldr::keyword(external.name.span.slice(source))
                }
                Expression::Command => DbgDocBldr::keyword(self.span.slice(source)),
                Expression::Boolean(boolean) => match boolean {
                    true => DbgDocBldr::primitive("$yes"),
                    false => DbgDocBldr::primitive("$no"),
                },
            },
        }
    }

    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match &self.expr {
            Expression::Literal(literal) => {
                literal.clone().into_spanned(self.span).pretty_debug(source)
            }
            Expression::ExternalWord => DbgDocBldr::typed(
                "external word",
                DbgDocBldr::primitive(self.span.slice(source)),
            ),
            Expression::Synthetic(s) => match s {
                Synthetic::String(s) => {
                    DbgDocBldr::typed("synthetic", DbgDocBldr::primitive(format!("{:?}", s)))
                }
            },
            Expression::Variable(_, _) => DbgDocBldr::keyword(self.span.slice(source)),
            Expression::Binary(binary) => binary.pretty_debug(source),
            Expression::Range(range) => range.pretty_debug(source),
            Expression::Block(_) => DbgDocBldr::opaque("block"),
            Expression::Invocation(_) => DbgDocBldr::opaque("invocation"),
            Expression::Garbage => DbgDocBldr::opaque("garbage"),
            Expression::List(list) => DbgDocBldr::delimit(
                "[",
                DbgDocBldr::intersperse(
                    list.iter().map(|item| item.pretty_debug(source)),
                    DbgDocBldr::space(),
                ),
                "]",
            ),
            Expression::Table(_headers, cells) => DbgDocBldr::delimit(
                "[",
                DbgDocBldr::intersperse(
                    cells
                        .iter()
                        .map(|row| row.iter().map(|item| item.pretty_debug(source)))
                        .flatten(),
                    DbgDocBldr::space(),
                ),
                "]",
            ),
            Expression::Path(path) => path.pretty_debug(source),
            Expression::FilePath(path) => {
                DbgDocBldr::typed("path", DbgDocBldr::primitive(path.display()))
            }
            Expression::ExternalCommand(external) => DbgDocBldr::typed(
                "command",
                DbgDocBldr::keyword("^") + DbgDocBldr::primitive(external.name.span.slice(source)),
            ),
            Expression::Command => {
                DbgDocBldr::typed("command", DbgDocBldr::primitive(self.span.slice(source)))
            }
            Expression::Boolean(boolean) => match boolean {
                true => DbgDocBldr::primitive("$yes"),
                false => DbgDocBldr::primitive("$no"),
            },
        }
    }
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
    Modulo,
    And,
    Or,
    Pow,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Deserialize, Serialize, new)]
pub struct Binary {
    pub left: SpannedExpression,
    pub op: SpannedExpression,
    pub right: SpannedExpression,
}

impl PrettyDebugWithSource for Binary {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        DbgDocBldr::delimit(
            "<",
            self.left.pretty_debug(source)
                + DbgDocBldr::space()
                + DbgDocBldr::keyword(self.op.span.slice(source))
                + DbgDocBldr::space()
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
    pub left: Option<SpannedExpression>,
    pub operator: Spanned<RangeOperator>,
    pub right: Option<SpannedExpression>,
}

impl PrettyDebugWithSource for Range {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        DbgDocBldr::delimit(
            "<",
            (if let Some(left) = &self.left {
                left.pretty_debug(source)
            } else {
                DebugDocBuilder::blank()
            }) + DbgDocBldr::space()
                + DbgDocBldr::keyword(self.operator.span().slice(source))
                + DbgDocBldr::space()
                + (if let Some(right) = &self.right {
                    right.pretty_debug(source)
                } else {
                    DebugDocBuilder::blank()
                }),
            ">",
        )
        .group()
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Deserialize, Serialize)]
pub enum RangeOperator {
    Inclusive,
    RightExclusive,
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
                Literal::String(string) => DbgDocBldr::primitive(format!("{:?}", string)), //string.slice(source))),
                Literal::GlobPattern(pattern) => DbgDocBldr::primitive(pattern),
                Literal::ColumnPath(path) => {
                    DbgDocBldr::intersperse_with_source(path.iter(), DbgDocBldr::space(), source)
                }
                Literal::Bare(bare) => {
                    DbgDocBldr::delimit("b\"", DbgDocBldr::primitive(bare), "\"")
                }
                Literal::Operator(operator) => DbgDocBldr::primitive(format!("{:?}", operator)),
            },
        }
    }

    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match &self.literal {
            Literal::Number(number) => number.pretty(),
            Literal::Size(number, unit) => {
                DbgDocBldr::typed("size", (number.pretty() + unit.pretty()).group())
            }
            Literal::String(string) => DbgDocBldr::typed(
                "string",
                DbgDocBldr::primitive(format!("{:?}", string)), //string.slice(source))),
            ),
            Literal::GlobPattern(pattern) => {
                DbgDocBldr::typed("pattern", DbgDocBldr::primitive(pattern))
            }
            Literal::ColumnPath(path) => DbgDocBldr::typed(
                "column path",
                DbgDocBldr::intersperse_with_source(path.iter(), DbgDocBldr::space(), source),
            ),
            Literal::Bare(bare) => DbgDocBldr::typed("bare", DbgDocBldr::primitive(bare)),
            Literal::Operator(operator) => {
                DbgDocBldr::typed("operator", DbgDocBldr::primitive(format!("{:?}", operator)))
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
            + DbgDocBldr::operator(".")
            + DbgDocBldr::intersperse(
                self.tail.iter().map(|m| m.pretty()),
                DbgDocBldr::operator("."),
            )
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Deserialize, Serialize)]
pub enum Expression {
    Literal(Literal),
    ExternalWord,
    Synthetic(Synthetic),
    Variable(String, Span),
    Binary(Box<Binary>),
    Range(Box<Range>),
    Block(hir::Block),
    List(Vec<SpannedExpression>),
    Table(Vec<SpannedExpression>, Vec<Vec<SpannedExpression>>),
    Path(Box<Path>),

    FilePath(PathBuf),
    ExternalCommand(ExternalStringCommand),
    Command,
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
            Expression::Command => "command",
            Expression::ExternalWord => "external word",
            Expression::FilePath(..) => "file path",
            Expression::Variable(..) => "variable",
            Expression::List(..) => "list",
            Expression::Table(..) => "table",
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
    pub fn integer(i: BigInt) -> Expression {
        Expression::Literal(Literal::Number(Number::Int(i)))
    }

    pub fn decimal(dec: BigDecimal) -> Expression {
        Expression::Literal(Literal::Number(Number::Decimal(dec)))
    }

    pub fn string(s: String) -> Expression {
        Expression::Literal(Literal::String(s))
    }

    pub fn operator(operator: Operator) -> Expression {
        Expression::Literal(Literal::Operator(operator))
    }

    pub fn range(
        left: Option<SpannedExpression>,
        operator: Spanned<RangeOperator>,
        right: Option<SpannedExpression>,
    ) -> Expression {
        Expression::Range(Box::new(Range {
            left,
            operator,
            right,
        }))
    }

    pub fn glob_pattern(p: String) -> Expression {
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
        Expression::Variable(v, span)
    }

    pub fn boolean(b: bool) -> Expression {
        Expression::Boolean(b)
    }

    pub fn has_it_usage(&self) -> bool {
        match self {
            Expression::Variable(name, _) if name == "$it" => true,
            Expression::Table(headers, values) => {
                headers.iter().any(|se| se.has_it_usage())
                    || values.iter().any(|v| v.iter().any(|se| se.has_it_usage()))
            }
            Expression::List(list) => list.iter().any(|se| se.has_it_usage()),
            Expression::Invocation(block) => block.has_it_usage(),
            Expression::Binary(binary) => binary.left.has_it_usage() || binary.right.has_it_usage(),
            Expression::Path(path) => path.head.has_it_usage(),
            Expression::Range(range) => {
                (if let Some(left) = &range.left {
                    left.has_it_usage()
                } else {
                    false
                }) || (if let Some(right) = &range.right {
                    right.has_it_usage()
                } else {
                    false
                })
            }
            _ => false,
        }
    }

    pub fn get_free_variables(&self, known_variables: &mut Vec<String>) -> Vec<String> {
        let mut output = vec![];
        match self {
            Expression::Variable(name, _) => {
                if !known_variables.contains(name) {
                    output.push(name.clone());
                }
            }
            Expression::Table(headers, values) => {
                for header in headers {
                    output.extend(header.get_free_variables(known_variables));
                }
                for row in values {
                    for value in row {
                        output.extend(value.get_free_variables(known_variables));
                    }
                }
            }
            Expression::List(list) => {
                for item in list {
                    output.extend(item.get_free_variables(known_variables));
                }
            }
            Expression::Invocation(block) | Expression::Block(block) => {
                output.extend(block.get_free_variables(known_variables));
            }
            Expression::Binary(binary) => {
                output.extend(binary.left.get_free_variables(known_variables));
                output.extend(binary.right.get_free_variables(known_variables));
            }
            Expression::Path(path) => {
                output.extend(path.head.get_free_variables(known_variables));
            }
            Expression::Range(range) => {
                if let Some(left) = &range.left {
                    output.extend(left.get_free_variables(known_variables));
                }
                if let Some(right) = &range.right {
                    output.extend(right.get_free_variables(known_variables));
                }
            }
            _ => {}
        }
        output
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum NamedValue {
    AbsentSwitch,
    PresentSwitch(Span),
    AbsentValue,
    Value(Span, Box<SpannedExpression>),
}

impl NamedValue {
    fn has_it_usage(&self) -> bool {
        if let NamedValue::Value(_, se) = self {
            se.has_it_usage()
        } else {
            false
        }
    }
    pub fn get_free_variables(&self, known_variables: &mut Vec<String>) -> Vec<String> {
        if let NamedValue::Value(_, se) = self {
            se.get_free_variables(known_variables)
        } else {
            vec![]
        }
    }
}

impl PrettyDebugWithSource for NamedValue {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self {
            NamedValue::AbsentSwitch => {
                DbgDocBldr::typed("switch", DbgDocBldr::description("absent"))
            }
            NamedValue::PresentSwitch(_) => {
                DbgDocBldr::typed("switch", DbgDocBldr::description("present"))
            }
            NamedValue::AbsentValue => DbgDocBldr::description("absent"),
            NamedValue::Value(_, value) => value.pretty_debug(source),
        }
    }

    fn refined_pretty_debug(&self, refine: PrettyDebugRefineKind, source: &str) -> DebugDocBuilder {
        match refine {
            PrettyDebugRefineKind::ContextFree => self.pretty_debug(source),
            PrettyDebugRefineKind::WithContext => match self {
                NamedValue::AbsentSwitch => DbgDocBldr::value("absent"),
                NamedValue::PresentSwitch(_) => DbgDocBldr::value("present"),
                NamedValue::AbsentValue => DbgDocBldr::value("absent"),
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

    pub fn has_it_usage(&self) -> bool {
        self.head.has_it_usage()
            || (if let Some(pos) = &self.positional {
                pos.iter().any(|x| x.has_it_usage())
            } else {
                false
            })
            || (if let Some(named) = &self.named {
                named.has_it_usage()
            } else {
                false
            })
    }

    pub fn get_free_variables(&self, known_variables: &mut Vec<String>) -> Vec<String> {
        let mut free_variables = vec![];

        free_variables.extend(self.head.get_free_variables(known_variables));
        if let Some(pos) = &self.positional {
            for pos in pos {
                free_variables.extend(pos.get_free_variables(known_variables));
            }
        }

        if let Some(named) = &self.named {
            free_variables.extend(named.get_free_variables(known_variables));
        }

        free_variables
    }
}

impl PrettyDebugWithSource for Call {
    fn refined_pretty_debug(&self, refine: PrettyDebugRefineKind, source: &str) -> DebugDocBuilder {
        match refine {
            PrettyDebugRefineKind::ContextFree => self.pretty_debug(source),
            PrettyDebugRefineKind::WithContext => {
                self.head
                    .refined_pretty_debug(PrettyDebugRefineKind::WithContext, source)
                    + DbgDocBldr::preceded_option(
                        Some(DbgDocBldr::space()),
                        self.positional.as_ref().map(|pos| {
                            DbgDocBldr::intersperse(
                                pos.iter().map(|expr| {
                                    expr.refined_pretty_debug(
                                        PrettyDebugRefineKind::WithContext,
                                        source,
                                    )
                                }),
                                DbgDocBldr::space(),
                            )
                        }),
                    )
                    + DbgDocBldr::preceded_option(
                        Some(DbgDocBldr::space()),
                        self.named.as_ref().map(|named| {
                            named.refined_pretty_debug(PrettyDebugRefineKind::WithContext, source)
                        }),
                    )
            }
        }
    }

    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        DbgDocBldr::typed(
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
    DotDotLeftAngleBracket,
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
        let that: Vec<&NamedValue> = other.named.values().collect();

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
        let that: Vec<&NamedValue> = other.named.values().collect();

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

    pub fn has_it_usage(&self) -> bool {
        self.iter().any(|x| x.1.has_it_usage())
    }

    pub fn get_free_variables(&self, known_variables: &mut Vec<String>) -> Vec<String> {
        let mut free_variables = vec![];
        for (_, val) in self.named.iter() {
            free_variables.extend(val.get_free_variables(known_variables));
        }
        free_variables
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
                .insert(name.into(), NamedValue::Value(flag_span, Box::new(expr))),
        };
    }

    pub fn insert_mandatory(
        &mut self,
        name: impl Into<String>,
        flag_span: Span,
        expr: SpannedExpression,
    ) {
        self.named
            .insert(name.into(), NamedValue::Value(flag_span, Box::new(expr)));
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
            PrettyDebugRefineKind::WithContext => DbgDocBldr::intersperse(
                self.named.iter().map(|(key, value)| {
                    DbgDocBldr::key(key)
                        + DbgDocBldr::equals()
                        + value.refined_pretty_debug(PrettyDebugRefineKind::WithContext, source)
                }),
                DbgDocBldr::space(),
            ),
        }
    }

    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        DbgDocBldr::delimit(
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
            FlagKind::Longhand => DbgDocBldr::description("--"),
            FlagKind::Shorthand => DbgDocBldr::description("-"),
        };

        prefix + DbgDocBldr::description(self.name.slice(source))
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
