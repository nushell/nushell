use super::Expression;
use crate::{Range, Span, Value, VarId};
use serde::{Deserialize, Serialize};
use std::ops::Bound;

/// One arm of a `match` block: `pattern if guard => body`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    /// The expression after `if`. The arm is taken only when it's true.
    pub guard: Option<Box<Expression>>,
    pub body: Expression,
}

impl MatchArm {
    /// Whether the arm matches every value: a wildcard pattern with no guard.
    pub fn is_wildcard(&self) -> bool {
        self.guard.is_none() && self.pattern.is_wildcard()
    }
}

/// A pattern with its span, in a match arm or nested in another pattern
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchPattern {
    pub pattern: Pattern,
    pub span: Span,
}

impl MatchPattern {
    pub fn variables(&self) -> Vec<VarId> {
        self.pattern.variables()
    }

    pub fn is_wildcard(&self) -> bool {
        self.pattern.is_wildcard()
    }
}

/// AST Node for pattern matching rules
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    /// Destructuring of records. A `..$rest` entry is a [`Pattern::Rest`] with an empty field
    /// name; it binds a record of the fields the other entries don't name.
    Record(Vec<(String, MatchPattern)>),
    /// List destructuring
    List(Vec<MatchPattern>),
    /// Matching against a literal (pure value), including const-evaluated expressions.
    Value(Value),
    /// A range, as in `1..5`, `1..<5`, `..0` or `1..3..9`: a number between its ends. The step
    /// doesn't matter, and `low` is the smaller end whichever way the range runs.
    Interval {
        low: Bound<Value>,
        high: Bound<Value>,
    },
    /// binding to a variable
    Variable(VarId),
    /// the `pattern1 \ pattern2` or-pattern
    Or(Vec<MatchPattern>),
    /// the `..$foo` pattern
    Rest(VarId),
    /// the `..` pattern
    IgnoreRest,
    /// the `pattern?` and `pattern = default` patterns: a record field or list item that may be
    /// missing. A missing or `null` value matches `pattern` against the constant `default`, or
    /// without one binds every variable inside it to `null`.
    Optional {
        pattern: Box<MatchPattern>,
        default: Option<Box<Value>>,
    },
    /// the `_` pattern
    IgnoreValue,
    /// Failed parsing of a pattern
    Garbage,
}

impl Pattern {
    pub fn variables(&self) -> Vec<VarId> {
        let mut output = vec![];
        match self {
            Pattern::Record(items) => {
                for item in items {
                    output.append(&mut item.1.variables());
                }
            }
            Pattern::List(items) => {
                for item in items {
                    output.append(&mut item.variables());
                }
            }
            Pattern::Variable(var_id) => output.push(*var_id),
            Pattern::Or(patterns) => {
                for pattern in patterns {
                    output.append(&mut pattern.variables());
                }
            }
            Pattern::Rest(var_id) => output.push(*var_id),
            Pattern::Optional { pattern, .. } => output.append(&mut pattern.variables()),
            Pattern::Value(_)
            | Pattern::Interval { .. }
            | Pattern::IgnoreValue
            | Pattern::Garbage
            | Pattern::IgnoreRest => {}
        }

        output
    }

    /// The pattern for a range: the numbers between its ends. `open_start` is for a range
    /// written without a start, like `..0`, which then has no lower end; a range value starts
    /// such a range at 0.
    pub fn interval(range: &Range, open_start: bool, span: Span) -> Self {
        let (start, ascending, end) = match range {
            Range::IntRange(range) => (
                Value::int(range.start(), span),
                range.step() > 0,
                range.end().map(|end| Value::int(end, span)),
            ),
            Range::FloatRange(range) => (
                Value::float(range.start(), span),
                range.step() > 0.0,
                range.end().map(|end| Value::float(end, span)),
            ),
        };
        let (low, high) = if open_start {
            (Bound::Unbounded, end)
        } else if ascending {
            (Bound::Included(start), end)
        } else {
            (end, Bound::Included(start))
        };
        Pattern::Interval { low, high }
    }

    pub fn is_wildcard(&self) -> bool {
        match self {
            Self::Variable(_) | Self::IgnoreValue => true,
            Self::Or(match_patterns) => match_patterns.iter().any(|x| x.is_wildcard()),
            Self::Optional { pattern, .. } => pattern.is_wildcard(),
            _ => false,
        }
    }
}
