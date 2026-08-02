use super::Expression;
use crate::{Span, Value, VarId};
use serde::{Deserialize, Serialize};

/// AST Node for match arm with optional match guard
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchPattern {
    pub pattern: Pattern,
    pub guard: Option<Box<Expression>>,
    pub span: Span,
}

impl MatchPattern {
    pub fn variables(&self) -> Vec<VarId> {
        self.pattern.variables()
    }

    pub fn is_wildcard(&self) -> bool {
        self.guard.is_none() && self.pattern.is_wildcard()
    }
}

/// AST Node for pattern matching rules
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    /// Destructuring of records
    Record(Vec<(String, MatchPattern)>),
    /// List destructuring
    List(Vec<MatchPattern>),
    /// Matching against a literal (from expression result).
    /// Prefer [`Pattern::Value`] for new patterns; the parser const-evaluates
    /// literal / parenthesized arms into `Value` when possible.
    Expression(Box<Expression>),
    /// Matching against a literal (pure value), including const-evaluated expressions.
    /// Range values match by containment rather than equality.
    Value(Value),
    /// binding to a variable
    Variable(VarId),
    /// the `pattern1 \ pattern2` or-pattern
    Or(Vec<MatchPattern>),
    /// the `..$foo` pattern
    Rest(VarId),
    /// the `..` pattern
    IgnoreRest,
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
            Pattern::Expression(_)
            | Pattern::Value(_)
            | Pattern::IgnoreValue
            | Pattern::Garbage
            | Pattern::IgnoreRest => {}
        }

        output
    }

    pub fn is_wildcard(&self) -> bool {
        match self {
            Self::Variable(_) | Self::IgnoreValue => true,
            Self::Or(match_patterns) => match_patterns.iter().any(|x| x.is_wildcard()),
            _ => false,
        }
    }
}
