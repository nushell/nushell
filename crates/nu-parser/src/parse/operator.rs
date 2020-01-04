use nu_source::{b, DebugDocBuilder, PrettyDebug};
use serde::{Deserialize, Serialize};

use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
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

impl PrettyDebug for CompareOperator {
    fn pretty(&self) -> DebugDocBuilder {
        b::operator(self.as_str())
    }
}

impl CompareOperator {
    pub fn print(self) -> String {
        self.as_str().to_string()
    }

    pub fn as_str(self) -> &'static str {
        match self {
            CompareOperator::Equal => "==",
            CompareOperator::NotEqual => "!=",
            CompareOperator::LessThan => "<",
            CompareOperator::GreaterThan => ">",
            CompareOperator::LessThanOrEqual => "<=",
            CompareOperator::GreaterThanOrEqual => ">=",
            CompareOperator::Contains => "=~",
            CompareOperator::NotContains => "!~",
        }
    }
}

impl From<&str> for CompareOperator {
    fn from(input: &str) -> CompareOperator {
        if let Ok(output) = CompareOperator::from_str(input) {
            output
        } else {
            unreachable!("Internal error: CompareOperator from failed")
        }
    }
}

impl FromStr for CompareOperator {
    type Err = ();
    fn from_str(input: &str) -> Result<Self, <Self as std::str::FromStr>::Err> {
        match input {
            "==" => Ok(CompareOperator::Equal),
            "!=" => Ok(CompareOperator::NotEqual),
            "<" => Ok(CompareOperator::LessThan),
            ">" => Ok(CompareOperator::GreaterThan),
            "<=" => Ok(CompareOperator::LessThanOrEqual),
            ">=" => Ok(CompareOperator::GreaterThanOrEqual),
            "=~" => Ok(CompareOperator::Contains),
            "!~" => Ok(CompareOperator::NotContains),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum EvaluationOperator {
    Dot,
    DotDot,
}

impl PrettyDebug for EvaluationOperator {
    fn pretty(&self) -> DebugDocBuilder {
        b::operator(self.as_str())
    }
}

impl EvaluationOperator {
    pub fn print(self) -> String {
        self.as_str().to_string()
    }

    pub fn as_str(self) -> &'static str {
        match self {
            EvaluationOperator::Dot => ".",
            EvaluationOperator::DotDot => "..",
        }
    }
}

impl From<&str> for EvaluationOperator {
    fn from(input: &str) -> EvaluationOperator {
        if let Ok(output) = EvaluationOperator::from_str(input) {
            output
        } else {
            unreachable!("Internal error: EvaluationOperator 'from' failed")
        }
    }
}

impl FromStr for EvaluationOperator {
    type Err = ();
    fn from_str(input: &str) -> Result<Self, <Self as std::str::FromStr>::Err> {
        match input {
            "." => Ok(EvaluationOperator::Dot),
            ".." => Ok(EvaluationOperator::DotDot),
            _ => Err(()),
        }
    }
}
