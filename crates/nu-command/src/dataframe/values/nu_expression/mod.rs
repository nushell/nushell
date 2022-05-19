mod custom_value;

use core::fmt;
use nu_protocol::{PipelineData, ShellError, Span, Value};
use polars::prelude::{col, AggExpr, Expr, Literal};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Polars Expression wrapper for Nushell operations
// Object is behind and Option to allow easy implementation of
// the Deserialize trait
#[derive(Default, Clone)]
pub struct NuExpression(Option<Expr>);

// Mocked serialization of the LazyFrame object
impl Serialize for NuExpression {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_none()
    }
}

// Mocked deserialization of the LazyFrame object
impl<'de> Deserialize<'de> for NuExpression {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(NuExpression::default())
    }
}

impl fmt::Debug for NuExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NuExpression")
    }
}

// Referenced access to the real LazyFrame
impl AsRef<Expr> for NuExpression {
    fn as_ref(&self) -> &polars::prelude::Expr {
        // The only case when there cannot be an expr is if it is created
        // using the default function or if created by deserializing something
        self.0.as_ref().expect("there should always be a frame")
    }
}

impl AsMut<Expr> for NuExpression {
    fn as_mut(&mut self) -> &mut polars::prelude::Expr {
        // The only case when there cannot be an expr is if it is created
        // using the default function or if created by deserializing something
        self.0.as_mut().expect("there should always be a frame")
    }
}

impl From<Expr> for NuExpression {
    fn from(expr: Expr) -> Self {
        Self(Some(expr))
    }
}

impl NuExpression {
    pub fn into_value(self, span: Span) -> Value {
        Value::CustomValue {
            val: Box::new(self),
            span,
        }
    }

    pub fn try_from_value(value: Value) -> Result<Self, ShellError> {
        match value {
            Value::CustomValue { val, span } => match val.as_any().downcast_ref::<Self>() {
                Some(expr) => Ok(NuExpression(expr.0.clone())),
                None => Err(ShellError::CantConvert(
                    "lazy expression".into(),
                    "non-dataframe".into(),
                    span,
                    None,
                )),
            },
            Value::String { val, .. } => Ok(col(val.as_str()).into()),
            Value::Int { val, .. } => Ok(val.lit().into()),
            Value::Bool { val, .. } => Ok(val.lit().into()),
            Value::Float { val, .. } => Ok(val.lit().into()),
            x => Err(ShellError::CantConvert(
                "lazy expression".into(),
                x.get_type().to_string(),
                x.span()?,
                None,
            )),
        }
    }

    pub fn try_from_pipeline(input: PipelineData, span: Span) -> Result<Self, ShellError> {
        let value = input.into_value(span);
        Self::try_from_value(value)
    }

    pub fn can_downcast(value: &Value) -> bool {
        match value {
            Value::CustomValue { val, .. } => val.as_any().downcast_ref::<Self>().is_some(),
            Value::String { .. } | Value::Int { .. } | Value::Bool { .. } | Value::Float { .. } => {
                true
            }
            _ => false,
        }
    }

    pub fn into_polars(self) -> Expr {
        self.0.expect("Expression cannot be none to convert")
    }

    pub fn apply_with_expr<F>(self, other: NuExpression, f: F) -> Self
    where
        F: Fn(Expr, Expr) -> Expr,
    {
        let expr = self.0.expect("Lazy expression must not be empty to apply");
        let other = other.0.expect("Lazy expression must not be empty to apply");

        f(expr, other).into()
    }

    pub fn to_value(&self, span: Span) -> Value {
        expr_to_value(self.as_ref(), span)
    }

    // Convenient function to extrac multiple Expr that could be inside a nushell Value
    pub fn extract_exprs(value: Value) -> Result<Vec<Expr>, ShellError> {
        ExtractedExpr::extract_exprs(value).map(ExtractedExpr::into_exprs)
    }
}

// Enum to represent the parsing of the expressions from Value
enum ExtractedExpr {
    Single(Expr),
    List(Vec<ExtractedExpr>),
}

impl ExtractedExpr {
    fn into_exprs(self) -> Vec<Expr> {
        match self {
            Self::Single(expr) => vec![expr],
            Self::List(expressions) => expressions
                .into_iter()
                .flat_map(ExtractedExpr::into_exprs)
                .collect(),
        }
    }

    fn extract_exprs(value: Value) -> Result<ExtractedExpr, ShellError> {
        match value {
            Value::String { val, .. } => Ok(ExtractedExpr::Single(col(val.as_str()))),
            Value::CustomValue { .. } => NuExpression::try_from_value(value)
                .map(NuExpression::into_polars)
                .map(ExtractedExpr::Single),
            Value::List { vals, .. } => vals
                .into_iter()
                .map(Self::extract_exprs)
                .collect::<Result<Vec<ExtractedExpr>, ShellError>>()
                .map(ExtractedExpr::List),
            x => Err(ShellError::CantConvert(
                "expression".into(),
                x.get_type().to_string(),
                x.span()?,
                None,
            )),
        }
    }
}

pub fn expr_to_value(expr: &Expr, span: Span) -> Value {
    let cols = vec!["expr".to_string(), "value".to_string()];

    match expr {
        Expr::Not(_) => todo!(),
        Expr::Alias(expr, alias) => {
            let expr = expr_to_value(expr.as_ref(), span);
            let alias = Value::String {
                val: alias.as_ref().into(),
                span,
            };

            let cols = vec!["expr".to_string(), "alias".to_string()];

            Value::Record {
                cols,
                vals: vec![expr, alias],
                span,
            }
        }
        Expr::Column(name) => {
            let expr_type = Value::String {
                val: "column".into(),
                span,
            };
            let value = Value::String {
                val: name.to_string(),
                span,
            };

            let vals = vec![expr_type, value];
            Value::Record { cols, vals, span }
        }
        Expr::Columns(columns) => {
            let expr_type = Value::String {
                val: "columns".into(),
                span,
            };
            let value = Value::List {
                vals: columns
                    .iter()
                    .map(|col| Value::String {
                        val: col.clone(),
                        span,
                    })
                    .collect(),
                span,
            };

            let vals = vec![expr_type, value];
            Value::Record { cols, vals, span }
        }
        Expr::DtypeColumn(_) => todo!(),
        Expr::Literal(literal) => {
            let expr_type = Value::String {
                val: "literal".into(),
                span,
            };
            let value = Value::String {
                val: format!("{:?}", literal),
                span,
            };

            let vals = vec![expr_type, value];
            Value::Record { cols, vals, span }
        }
        Expr::BinaryExpr { left, op, right } => {
            let left_val = expr_to_value(left, span);
            let right_val = expr_to_value(right, span);

            let operator = Value::String {
                val: format!("{:?}", op),
                span,
            };

            let cols = vec!["left".to_string(), "op".to_string(), "right".to_string()];

            Value::Record {
                cols,
                vals: vec![left_val, operator, right_val],
                span,
            }
        }
        Expr::Ternary {
            predicate,
            truthy,
            falsy,
        } => {
            let predicate = expr_to_value(predicate.as_ref(), span);
            let truthy = expr_to_value(truthy.as_ref(), span);
            let falsy = expr_to_value(falsy.as_ref(), span);

            let cols = vec![
                "predicate".to_string(),
                "truthy".to_string(),
                "falsy".to_string(),
            ];

            Value::Record {
                cols,
                vals: vec![predicate, truthy, falsy],
                span,
            }
        }
        Expr::Agg(agg_expr) => {
            let value = match agg_expr {
                AggExpr::Min(expr)
                | AggExpr::Max(expr)
                | AggExpr::Median(expr)
                | AggExpr::NUnique(expr)
                | AggExpr::First(expr)
                | AggExpr::Last(expr)
                | AggExpr::Mean(expr)
                | AggExpr::List(expr)
                | AggExpr::Count(expr)
                | AggExpr::Sum(expr)
                | AggExpr::AggGroups(expr)
                | AggExpr::Std(expr)
                | AggExpr::Var(expr) => expr_to_value(expr.as_ref(), span),
                AggExpr::Quantile { .. } => todo!(),
            };

            let expr_type = Value::String {
                val: "agg".into(),
                span,
            };

            let vals = vec![expr_type, value];
            Value::Record { cols, vals, span }
        }
        Expr::IsNotNull(_) => todo!(),
        Expr::IsNull(_) => todo!(),
        Expr::Cast { .. } => todo!(),
        Expr::Sort { .. } => todo!(),
        Expr::Take { .. } => todo!(),
        Expr::SortBy { .. } => todo!(),
        Expr::Function { .. } => todo!(),
        Expr::Shift { .. } => todo!(),
        Expr::Reverse(_) => todo!(),
        Expr::Duplicated(_) => todo!(),
        Expr::IsUnique(_) => todo!(),
        Expr::Explode(_) => todo!(),
        Expr::Filter { .. } => todo!(),
        Expr::Window { .. } => todo!(),
        Expr::Wildcard => todo!(),
        Expr::Slice { .. } => todo!(),
        Expr::Exclude(_, _) => todo!(),
        Expr::KeepName(_) => todo!(),
        Expr::RenameAlias { .. } => todo!(),
        Expr::Count => todo!(),
        Expr::Nth(_) => todo!(),
        Expr::AnonymousFunction { .. } => todo!(),
    }
}
