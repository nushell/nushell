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
            Value::String { val, .. } => Ok(val.lit().into()),
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
            Value::List { vals, .. } => vals.iter().all(Self::can_downcast),
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
        Expr::Alias(expr, alias) => {
            let expr = expr_to_value(expr.as_ref(), span);
            let alias = Value::String {
                val: alias.as_ref().into(),
                span,
            };

            let cols = vec!["expr".into(), "alias".into()];

            Value::Record {
                cols,
                vals: vec![expr, alias],
                span,
            }
        }
        Expr::Column(name) => {
            let expr_type = Value::String {
                val: "column".to_string(),
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

            let cols = vec!["left".into(), "op".into(), "right".into()];

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

            let cols = vec!["predicate".into(), "truthy".into(), "falsy".into()];

            Value::Record {
                cols,
                vals: vec![predicate, truthy, falsy],
                span,
            }
        }
        Expr::Agg(agg_expr) => {
            let value = match agg_expr {
                AggExpr::Min { input: expr, .. }
                | AggExpr::Max { input: expr, .. }
                | AggExpr::Median(expr)
                | AggExpr::NUnique(expr)
                | AggExpr::First(expr)
                | AggExpr::Last(expr)
                | AggExpr::Mean(expr)
                | AggExpr::List(expr)
                | AggExpr::Count(expr)
                | AggExpr::Sum(expr)
                | AggExpr::AggGroups(expr)
                | AggExpr::Std(expr, _)
                | AggExpr::Var(expr, _) => expr_to_value(expr.as_ref(), span),
                AggExpr::Quantile {
                    expr,
                    quantile,
                    interpol,
                } => {
                    let expr = expr_to_value(expr.as_ref(), span);
                    let quantile = Value::Float {
                        val: *quantile,
                        span,
                    };
                    let interpol = Value::String {
                        val: format!("{:?}", interpol),
                        span,
                    };

                    let cols = vec!["expr".into(), "quantile".into(), "interpol".into()];

                    Value::Record {
                        cols,
                        vals: vec![expr, quantile, interpol],
                        span,
                    }
                }
            };

            let expr_type = Value::String {
                val: "agg".into(),
                span,
            };

            let vals = vec![expr_type, value];
            Value::Record { cols, vals, span }
        }
        Expr::Count => {
            let expr = Value::String {
                val: "count".into(),
                span,
            };
            let cols = vec!["expr".into()];

            Value::Record {
                cols,
                vals: vec![expr],
                span,
            }
        }
        Expr::Wildcard => {
            let expr = Value::String {
                val: "wildcard".into(),
                span,
            };
            let cols = vec!["expr".into()];

            Value::Record {
                cols,
                vals: vec![expr],
                span,
            }
        }
        Expr::Explode(expr) => {
            let expr = expr_to_value(expr.as_ref(), span);
            let cols = vec!["expr".into()];

            Value::Record {
                cols,
                vals: vec![expr],
                span,
            }
        }
        Expr::KeepName(expr) => {
            let expr = expr_to_value(expr.as_ref(), span);
            let cols = vec!["expr".into()];

            Value::Record {
                cols,
                vals: vec![expr],
                span,
            }
        }
        Expr::Nth(i) => {
            let expr = Value::int(*i, span);
            let cols = vec!["expr".into()];

            Value::Record {
                cols,
                vals: vec![expr],
                span,
            }
        }
        Expr::DtypeColumn(dtypes) => {
            let vals = dtypes
                .iter()
                .map(|d| Value::String {
                    val: format!("{}", d),
                    span,
                })
                .collect();

            Value::List { vals, span }
        }
        Expr::Sort { expr, options } => {
            let expr = expr_to_value(expr.as_ref(), span);
            let options = Value::String {
                val: format!("{:?}", options),
                span,
            };
            let cols = vec!["expr".into(), "options".into()];

            Value::Record {
                cols,
                vals: vec![expr, options],
                span,
            }
        }
        Expr::Cast {
            expr,
            data_type,
            strict,
        } => {
            let expr = expr_to_value(expr.as_ref(), span);
            let dtype = Value::String {
                val: format!("{:?}", data_type),
                span,
            };
            let strict = Value::Bool { val: *strict, span };

            let cols = vec!["expr".into(), "dtype".into(), "strict".into()];

            Value::Record {
                cols,
                vals: vec![expr, dtype, strict],
                span,
            }
        }
        Expr::Take { expr, idx } => {
            let expr = expr_to_value(expr.as_ref(), span);
            let idx = expr_to_value(idx.as_ref(), span);

            let cols = vec!["expr".into(), "idx".into()];

            Value::Record {
                cols,
                vals: vec![expr, idx],
                span,
            }
        }
        Expr::SortBy { expr, by, reverse } => {
            let expr = expr_to_value(expr.as_ref(), span);
            let by: Vec<Value> = by.iter().map(|b| expr_to_value(b, span)).collect();
            let by = Value::List { vals: by, span };

            let reverse: Vec<Value> = reverse
                .iter()
                .map(|r| Value::Bool { val: *r, span })
                .collect();
            let reverse = Value::List {
                vals: reverse,
                span,
            };

            let cols = vec!["expr".into(), "by".into(), "reverse".into()];

            Value::Record {
                cols,
                vals: vec![expr, by, reverse],
                span,
            }
        }
        Expr::Filter { input, by } => {
            let input = expr_to_value(input.as_ref(), span);
            let by = expr_to_value(by.as_ref(), span);

            let cols = vec!["input".into(), "by".into()];

            Value::Record {
                cols,
                vals: vec![input, by],
                span,
            }
        }
        Expr::Slice {
            input,
            offset,
            length,
        } => {
            let input = expr_to_value(input.as_ref(), span);
            let offset = expr_to_value(offset.as_ref(), span);
            let length = expr_to_value(length.as_ref(), span);

            let cols = vec!["input".into(), "offset".into(), "length".into()];

            Value::Record {
                cols,
                vals: vec![input, offset, length],
                span,
            }
        }
        Expr::Exclude(expr, excluded) => {
            let expr = expr_to_value(expr.as_ref(), span);
            let excluded = excluded
                .iter()
                .map(|e| Value::String {
                    val: format!("{:?}", e),
                    span,
                })
                .collect();
            let excluded = Value::List {
                vals: excluded,
                span,
            };

            let cols = vec!["expr".into(), "excluded".into()];

            Value::Record {
                cols,
                vals: vec![expr, excluded],
                span,
            }
        }
        Expr::RenameAlias { expr, function } => {
            let expr = expr_to_value(expr.as_ref(), span);
            let function = Value::String {
                val: format!("{:?}", function),
                span,
            };

            let cols = vec!["expr".into(), "function".into()];

            Value::Record {
                cols,
                vals: vec![expr, function],
                span,
            }
        }
        Expr::AnonymousFunction {
            input,
            function,
            output_type,
            options,
        } => {
            let input: Vec<Value> = input.iter().map(|e| expr_to_value(e, span)).collect();
            let input = Value::List { vals: input, span };

            let function = Value::String {
                val: format!("{:?}", function),
                span,
            };
            let output_type = Value::String {
                val: format!("{:?}", output_type),
                span,
            };
            let options = Value::String {
                val: format!("{:?}", options),
                span,
            };

            let cols = vec![
                "input".into(),
                "function".into(),
                "output_type".into(),
                "options".into(),
            ];

            Value::Record {
                cols,
                vals: vec![input, function, output_type, options],
                span,
            }
        }
        Expr::Function {
            input,
            function,
            options,
        } => {
            let input: Vec<Value> = input.iter().map(|e| expr_to_value(e, span)).collect();
            let input = Value::List { vals: input, span };

            let function = Value::String {
                val: format!("{:?}", function),
                span,
            };
            let options = Value::String {
                val: format!("{:?}", options),
                span,
            };

            let cols = vec!["input".into(), "function".into(), "options".into()];

            Value::Record {
                cols,
                vals: vec![input, function, options],
                span,
            }
        }
        Expr::Window {
            function,
            partition_by,
            order_by,
            options,
        } => {
            let function = expr_to_value(function, span);

            let partition_by: Vec<Value> = partition_by
                .iter()
                .map(|e| expr_to_value(e, span))
                .collect();
            let partition_by = Value::List {
                vals: partition_by,
                span,
            };

            let order_by = order_by
                .as_ref()
                .map(|e| expr_to_value(e.as_ref(), span))
                .unwrap_or_else(|| Value::nothing(span));

            let options = Value::String {
                val: format!("{:?}", options),
                span,
            };

            let cols = vec![
                "function".into(),
                "partition_by".into(),
                "order_by".into(),
                "options".into(),
            ];

            Value::Record {
                cols,
                vals: vec![function, partition_by, order_by, options],
                span,
            }
        }
    }
}
