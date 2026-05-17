use super::{Expr, Expression};
use crate::{ShellError, Span};
use serde::{Deserialize, Serialize};
use std::fmt;
use strum_macros::{EnumIter, EnumMessage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, EnumMessage)]
pub enum Comparison {
    #[strum(message = "Equal to")]
    Equal,
    #[strum(message = "Not equal to")]
    NotEqual,
    #[strum(message = "Less than")]
    LessThan,
    #[strum(message = "Greater than")]
    GreaterThan,
    #[strum(message = "Less than or equal to")]
    LessThanOrEqual,
    #[strum(message = "Greater than or equal to")]
    GreaterThanOrEqual,
    #[strum(message = "Contains regex match")]
    RegexMatch,
    #[strum(message = "Does not contain regex match")]
    NotRegexMatch,
    #[strum(message = "Is a member of (doesn't use regex)")]
    In,
    #[strum(message = "Is not a member of (doesn't use regex)")]
    NotIn,
    #[strum(message = "Contains a value of (doesn't use regex)")]
    Has,
    #[strum(message = "Does not contain a value of (doesn't use regex)")]
    NotHas,
    #[strum(message = "Starts with")]
    StartsWith,
    #[strum(message = "Does not start with")]
    NotStartsWith,
    #[strum(message = "Ends with")]
    EndsWith,
    #[strum(message = "Does not end with")]
    NotEndsWith,
}

impl Comparison {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::LessThan => "<",
            Self::GreaterThan => ">",
            Self::LessThanOrEqual => "<=",
            Self::GreaterThanOrEqual => ">=",
            Self::RegexMatch => "=~",
            Self::NotRegexMatch => "!~",
            Self::In => "in",
            Self::NotIn => "not-in",
            Self::Has => "has",
            Self::NotHas => "not-has",
            Self::StartsWith => "starts-with",
            Self::NotStartsWith => "not-starts-with",
            Self::EndsWith => "ends-with",
            Self::NotEndsWith => "not-ends-with",
        }
    }
}

impl AsRef<str> for Comparison {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, EnumMessage)]
pub enum Math {
    #[strum(message = "Add (Plus)")]
    Add,
    #[strum(message = "Subtract (Minus)")]
    Subtract,
    #[strum(message = "Multiply")]
    Multiply,
    #[strum(message = "Divide")]
    Divide,
    #[strum(message = "Floor division")]
    FloorDivide,
    #[strum(message = "Floor division remainder (Modulo)")]
    Modulo,
    #[strum(message = "Power of")]
    Pow,
    #[strum(message = "Concatenates two lists, two strings, or two binary values")]
    Concatenate,
}

impl Math {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::FloorDivide => "//",
            Self::Modulo => "mod",
            Self::Pow => "**",
            Self::Concatenate => "++",
        }
    }
}

impl AsRef<str> for Math {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Math {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, EnumMessage)]
pub enum Boolean {
    #[strum(message = "Logical OR (short-circuiting)")]
    Or,
    #[strum(message = "Logical XOR")]
    Xor,
    #[strum(message = "Logical AND (short-circuiting)")]
    And,
}

impl Boolean {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Or => "or",
            Self::Xor => "xor",
            Self::And => "and",
        }
    }
}

impl AsRef<str> for Boolean {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Boolean {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, EnumMessage)]
pub enum Bits {
    #[strum(message = "Bitwise OR")]
    BitOr,
    #[strum(message = "Bitwise exclusive OR")]
    BitXor,
    #[strum(message = "Bitwise AND")]
    BitAnd,
    #[strum(message = "Bitwise shift left")]
    ShiftLeft,
    #[strum(message = "Bitwise shift right")]
    ShiftRight,
}

impl AsRef<str> for Bits {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Bits {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::BitOr => "bit-or",
            Self::BitXor => "bit-xor",
            Self::BitAnd => "bit-and",
            Self::ShiftLeft => "bit-shl",
            Self::ShiftRight => "bit-shr",
        }
    }
}

impl fmt::Display for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, EnumMessage)]
pub enum Assignment {
    #[strum(message = "Assigns a value to a variable.")]
    Assign,
    #[strum(message = "Adds a value to a variable.")]
    AddAssign,
    #[strum(message = "Subtracts a value from a variable.")]
    SubtractAssign,
    #[strum(message = "Multiplies a variable by a value")]
    MultiplyAssign,
    #[strum(message = "Divides a variable by a value.")]
    DivideAssign,
    #[strum(message = "Concatenates a variable with a list, string or binary.")]
    ConcatenateAssign,
}

impl AsRef<str> for Assignment {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Assignment {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Assign => "=",
            Self::AddAssign => "+=",
            Self::SubtractAssign => "-=",
            Self::MultiplyAssign => "*=",
            Self::DivideAssign => "/=",
            Self::ConcatenateAssign => "++=",
        }
    }
}

impl fmt::Display for Assignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operator {
    Comparison(Comparison),
    Math(Math),
    Boolean(Boolean),
    Bits(Bits),
    Assignment(Assignment),
}

impl Operator {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Comparison(comparison) => comparison.as_str(),
            Self::Math(math) => math.as_str(),
            Self::Boolean(boolean) => boolean.as_str(),
            Self::Bits(bits) => bits.as_str(),
            Self::Assignment(assignment) => assignment.as_str(),
        }
    }

    pub const fn precedence(&self) -> u8 {
        match self {
            Self::Math(Math::Pow) => 100,
            Self::Math(Math::Multiply)
            | Self::Math(Math::Divide)
            | Self::Math(Math::Modulo)
            | Self::Math(Math::FloorDivide) => 95,
            Self::Math(Math::Add) | Self::Math(Math::Subtract) => 90,
            Self::Bits(Bits::ShiftLeft) | Self::Bits(Bits::ShiftRight) => 85,
            Self::Comparison(Comparison::NotRegexMatch)
            | Self::Comparison(Comparison::RegexMatch)
            | Self::Comparison(Comparison::StartsWith)
            | Self::Comparison(Comparison::NotStartsWith)
            | Self::Comparison(Comparison::EndsWith)
            | Self::Comparison(Comparison::NotEndsWith)
            | Self::Comparison(Comparison::LessThan)
            | Self::Comparison(Comparison::LessThanOrEqual)
            | Self::Comparison(Comparison::GreaterThan)
            | Self::Comparison(Comparison::GreaterThanOrEqual)
            | Self::Comparison(Comparison::Equal)
            | Self::Comparison(Comparison::NotEqual)
            | Self::Comparison(Comparison::In)
            | Self::Comparison(Comparison::NotIn)
            | Self::Comparison(Comparison::Has)
            | Self::Comparison(Comparison::NotHas)
            | Self::Math(Math::Concatenate) => 80,
            Self::Bits(Bits::BitAnd) => 75,
            Self::Bits(Bits::BitXor) => 70,
            Self::Bits(Bits::BitOr) => 60,
            Self::Boolean(Boolean::And) => 50,
            Self::Boolean(Boolean::Xor) => 45,
            Self::Boolean(Boolean::Or) => 40,
            Self::Assignment(_) => 10,
        }
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, EnumIter)]
pub enum RangeInclusion {
    Inclusive,
    RightExclusive,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RangeOperator {
    pub inclusion: RangeInclusion,
    pub span: Span,
    pub next_op_span: Span,
}

impl fmt::Display for RangeOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inclusion {
            RangeInclusion::Inclusive => write!(f, ".."),
            RangeInclusion::RightExclusive => write!(f, "..<"),
        }
    }
}

pub fn eval_operator(op: &Expression) -> Result<Operator, ShellError> {
    match op {
        Expression {
            expr: Expr::Operator(operator),
            ..
        } => Ok(*operator),
        Expression { span, expr, .. } => Err(ShellError::UnknownOperator {
            op_token: format!("{expr:?}"),
            span: *span,
        }),
    }
}
