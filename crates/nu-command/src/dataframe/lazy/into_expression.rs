/// Trait definition to extract all Polars Expr that may be contained
/// in a Nushell Value
use nu_protocol::{ShellError, Value};
use polars::prelude::Expr;

use crate::dataframe::values::NuExpression;

pub trait IntoExpression {
    fn into_expressions(self) -> Result<Vec<Expr>, ShellError>;
}

impl IntoExpression for Value {
    fn into_expressions(self) -> Result<Vec<Expr>, ShellError> {
        ExtractedExpr::extract_expressions(self).map(ExtractedExpr::into_expressions)
    }
}

// Enum to represent the parsing of the expressions from Value
enum ExtractedExpr {
    Single(Expr),
    List(Vec<ExtractedExpr>),
}

impl ExtractedExpr {
    fn into_expressions(self) -> Vec<Expr> {
        match self {
            Self::Single(expr) => vec![expr],
            Self::List(expressions) => expressions
                .into_iter()
                .flat_map(ExtractedExpr::into_expressions)
                .collect(),
        }
    }

    fn extract_expressions(value: Value) -> Result<ExtractedExpr, ShellError> {
        match value {
            Value::CustomValue { .. } => NuExpression::try_from_value(value)
                .map(NuExpression::into_polars)
                .map(ExtractedExpr::Single),
            Value::List { vals, .. } => vals
                .into_iter()
                .map(Self::extract_expressions)
                .collect::<Result<Vec<ExtractedExpr>, ShellError>>()
                .map(ExtractedExpr::List),
            x => Err(ShellError::CantConvert(
                "expression".into(),
                x.get_type().to_string(),
                x.span()?,
            )),
        }
    }
}
