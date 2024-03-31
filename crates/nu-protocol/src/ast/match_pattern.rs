use super::Expression;
use crate::{FutureSpanId, VarId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchPattern {
    pub pattern: Pattern,
    pub guard: Option<Expression>,
    pub span: FutureSpanId,
}

impl MatchPattern {
    pub fn variables(&self) -> Vec<VarId> {
        self.pattern.variables()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    Record(Vec<(String, MatchPattern)>),
    List(Vec<MatchPattern>),
    Value(Expression),
    Variable(VarId),
    Or(Vec<MatchPattern>),
    Rest(VarId), // the ..$foo pattern
    IgnoreRest,  // the .. pattern
    IgnoreValue, // the _ pattern
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
            Pattern::Value(_) | Pattern::IgnoreValue | Pattern::Garbage | Pattern::IgnoreRest => {}
        }

        output
    }
}
