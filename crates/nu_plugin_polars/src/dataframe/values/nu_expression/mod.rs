mod custom_value;

use nu_protocol::{ShellError, Span, Value, record};
use polars::{
    chunked_array::cast::CastOptions,
    prelude::{AggExpr, Expr, Literal, col},
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::{Cacheable, PolarsPlugin};

pub use self::custom_value::NuExpressionCustomValue;

use super::{CustomValueSupport, PolarsPluginObject, PolarsPluginType};

// Polars Expression wrapper for Nushell operations
// Object is behind and Option to allow easy implementation of
// the Deserialize trait
#[derive(Default, Clone, Debug)]
pub struct NuExpression {
    pub id: Uuid,
    expr: Option<Expr>,
}

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

// Referenced access to the real LazyFrame
impl AsRef<Expr> for NuExpression {
    fn as_ref(&self) -> &polars::prelude::Expr {
        // The only case when there cannot be an expr is if it is created
        // using the default function or if created by deserializing something
        self.expr.as_ref().expect("there should always be a frame")
    }
}

impl AsMut<Expr> for NuExpression {
    fn as_mut(&mut self) -> &mut polars::prelude::Expr {
        // The only case when there cannot be an expr is if it is created
        // using the default function or if created by deserializing something
        self.expr.as_mut().expect("there should always be a frame")
    }
}

impl From<Expr> for NuExpression {
    fn from(expr: Expr) -> Self {
        Self::new(Some(expr))
    }
}

impl NuExpression {
    fn new(expr: Option<Expr>) -> Self {
        Self {
            id: Uuid::new_v4(),
            expr,
        }
    }

    pub fn into_polars(self) -> Expr {
        self.expr.expect("Expression cannot be none to convert")
    }

    pub fn apply_with_expr<F>(self, other: NuExpression, f: F) -> Self
    where
        F: Fn(Expr, Expr) -> Expr,
    {
        let expr = self
            .expr
            .expect("Lazy expression must not be empty to apply");
        let other = other
            .expr
            .expect("Lazy expression must not be empty to apply");

        f(expr, other).into()
    }

    pub fn to_value(&self, span: Span) -> Result<Value, ShellError> {
        expr_to_value(self.as_ref(), span)
    }

    // Convenient function to extract multiple Expr that could be inside a nushell Value
    pub fn extract_exprs(plugin: &PolarsPlugin, value: Value) -> Result<Vec<Expr>, ShellError> {
        ExtractedExpr::extract_exprs(plugin, value).map(ExtractedExpr::into_exprs)
    }
}

#[derive(Debug)]
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

    fn extract_exprs(plugin: &PolarsPlugin, value: Value) -> Result<ExtractedExpr, ShellError> {
        match value {
            Value::String { val, .. } => Ok(ExtractedExpr::Single(col(val.as_str()))),
            Value::Custom { .. } => NuExpression::try_from_value(plugin, &value)
                .map(NuExpression::into_polars)
                .map(ExtractedExpr::Single),
            Value::Record { val, .. } => val
                .iter()
                .map(|(key, value)| {
                    NuExpression::try_from_value(plugin, value)
                        .map(NuExpression::into_polars)
                        .map(|expr| expr.alias(key))
                        .map(ExtractedExpr::Single)
                })
                .collect::<Result<Vec<ExtractedExpr>, ShellError>>()
                .map(ExtractedExpr::List),
            Value::List { vals, .. } => vals
                .into_iter()
                .map(|x| Self::extract_exprs(plugin, x))
                .collect::<Result<Vec<ExtractedExpr>, ShellError>>()
                .map(ExtractedExpr::List),
            x => Err(ShellError::CantConvert {
                to_type: "expression".into(),
                from_type: x.get_type().to_string(),
                span: x.span(),
                help: None,
            }),
        }
    }
}

pub fn expr_to_value(expr: &Expr, span: Span) -> Result<Value, ShellError> {
    match expr {
        Expr::Alias(expr, alias) => Ok(Value::record(
            record! {
                "expr" => expr_to_value(expr.as_ref(), span)?,
                "alias" => Value::string(alias.as_str(), span),
            },
            span,
        )),
        Expr::Column(name) => Ok(Value::record(
            record! {
                "expr" => Value::string("column", span),
                "value" => Value::string(name.to_string(), span),
            },
            span,
        )),
        Expr::Literal(literal) => Ok(Value::record(
            record! {
                "expr" => Value::string("literal", span),
                "value" => Value::string(format!("{literal:?}"), span),
            },
            span,
        )),
        Expr::BinaryExpr { left, op, right } => Ok(Value::record(
            record! {
                "left" => expr_to_value(left, span)?,
                "op" => Value::string(format!("{op:?}"), span),
                "right" => expr_to_value(right, span)?,
            },
            span,
        )),
        Expr::Ternary {
            predicate,
            truthy,
            falsy,
        } => Ok(Value::record(
            record! {
                "predicate" => expr_to_value(predicate.as_ref(), span)?,
                "truthy" => expr_to_value(truthy.as_ref(), span)?,
                "falsy" => expr_to_value(falsy.as_ref(), span)?,
            },
            span,
        )),
        Expr::Agg(agg_expr) => {
            let value = match agg_expr {
                AggExpr::Min { input: expr, .. }
                | AggExpr::Max { input: expr, .. }
                | AggExpr::Median(expr)
                | AggExpr::NUnique(expr)
                | AggExpr::First(expr)
                | AggExpr::Last(expr)
                | AggExpr::Mean(expr)
                | AggExpr::Implode(expr)
                | AggExpr::Count { input: expr, .. }
                | AggExpr::Sum(expr)
                | AggExpr::AggGroups(expr)
                | AggExpr::Std(expr, _)
                | AggExpr::Var(expr, _) => expr_to_value(expr.as_ref(), span),
                AggExpr::Quantile {
                    expr,
                    quantile,
                    method,
                } => Ok(Value::record(
                    record! {
                        "expr" => expr_to_value(expr.as_ref(), span)?,
                        "quantile" => expr_to_value(quantile.as_ref(), span)?,
                        "method" => Value::string(format!("{method:?}"), span),
                    },
                    span,
                )),
            };

            Ok(Value::record(
                record! {
                    "expr" => Value::string("agg", span),
                    "value" => value?,
                },
                span,
            ))
        }
        Expr::Len => Ok(Value::record(
            record! { "expr" => Value::string("count", span) },
            span,
        )),
        Expr::Explode { input: expr, .. } => Ok(Value::record(
            record! { "expr" => expr_to_value(expr.as_ref(), span)? },
            span,
        )),
        Expr::KeepName(expr) => Ok(Value::record(
            record! { "expr" => expr_to_value(expr.as_ref(), span)? },
            span,
        )),
        Expr::Sort { expr, options } => Ok(Value::record(
            record! {
                "expr" => expr_to_value(expr.as_ref(), span)?,
                "options" => Value::string(format!("{options:?}"), span),
            },
            span,
        )),
        Expr::Cast {
            expr,
            dtype: data_type,
            options,
        } => {
            let cast_option_str = match options {
                CastOptions::Strict => "STRICT",
                CastOptions::NonStrict => "NON_STRICT",
                CastOptions::Overflowing => "OVERFLOWING",
            };

            Ok(Value::record(
                record! {
                    "expr" => expr_to_value(expr.as_ref(), span)?,
                    "dtype" => Value::string(format!("{data_type:?}"), span),
                    "cast_options" => Value::string(cast_option_str, span)
                },
                span,
            ))
        }
        Expr::Gather {
            expr,
            idx,
            returns_scalar: _,
        } => Ok(Value::record(
            record! {
                "expr" => expr_to_value(expr.as_ref(), span)?,
                "idx" => expr_to_value(idx.as_ref(), span)?,
            },
            span,
        )),
        Expr::SortBy {
            expr,
            by,
            sort_options,
        } => {
            let by: Result<Vec<Value>, ShellError> =
                by.iter().map(|b| expr_to_value(b, span)).collect();
            let descending: Vec<Value> = sort_options
                .descending
                .iter()
                .map(|r| Value::bool(*r, span))
                .collect();

            Ok(Value::record(
                record! {
                    "expr" => expr_to_value(expr.as_ref(), span)?,
                    "by" => Value::list(by?, span),
                    "descending" => Value::list(descending, span),
                },
                span,
            ))
        }
        Expr::Filter { input, by } => Ok(Value::record(
            record! {
                "input" => expr_to_value(input.as_ref(), span)?,
                "by" => expr_to_value(by.as_ref(), span)?,
            },
            span,
        )),
        Expr::Slice {
            input,
            offset,
            length,
        } => Ok(Value::record(
            record! {
                "input" => expr_to_value(input.as_ref(), span)?,
                "offset" => expr_to_value(offset.as_ref(), span)?,
                "length" => expr_to_value(length.as_ref(), span)?,
            },
            span,
        )),
        Expr::RenameAlias { expr, .. } => Ok(Value::record(
            record! {
                "expr" => expr_to_value(expr.as_ref(), span)?,
            },
            span,
        )),
        Expr::AnonymousFunction {
            input,
            function,
            options,
            fmt_str,
        } => {
            let input: Result<Vec<Value>, ShellError> =
                input.iter().map(|e| expr_to_value(e, span)).collect();
            Ok(Value::record(
                record! {
                    "input" => Value::list(input?, span),
                    "function" => Value::string(format!("{function:?}"), span),
                    "options" => Value::string(format!("{options:?}"), span),
                    "fmt_str" => Value::string(format!("{fmt_str:?}"), span),
                },
                span,
            ))
        }
        Expr::Function { input, function } => {
            let input: Result<Vec<Value>, ShellError> =
                input.iter().map(|e| expr_to_value(e, span)).collect();
            Ok(Value::record(
                record! {
                    "input" => Value::list(input?, span),
                    "function" => Value::string(format!("{function:?}"), span),
                },
                span,
            ))
        }
        Expr::Window {
            function,
            partition_by,
            order_by,
            options,
        } => {
            let partition_by: Result<Vec<Value>, ShellError> = partition_by
                .iter()
                .map(|e| expr_to_value(e, span))
                .collect();

            Ok(Value::record(
                record! {
                    "function" => expr_to_value(function, span)?,
                    "partition_by" => Value::list(partition_by?, span),
                    "order_by" => {
                        if let Some((order_expr, sort_options)) = order_by {
                            Value::record(record! {
                                "expr" => expr_to_value(order_expr.as_ref(), span)?,
                                "sort_options" => {
                                    Value::record(record!(
                                        "descending" => Value::bool(sort_options.descending, span),
                                        "nulls_last"=> Value::bool(sort_options.nulls_last, span),
                                        "multithreaded"=> Value::bool(sort_options.multithreaded, span),
                                        "maintain_order"=> Value::bool(sort_options.maintain_order, span),
                                    ), span)
                                }
                            }, span)
                        } else {
                            Value::nothing(span)
                        }
                    },
                    "options" => Value::string(format!("{options:?}"), span),
                },
                span,
            ))
        }
        Expr::SubPlan(_, _) => Err(ShellError::UnsupportedInput {
            msg: "Expressions of type SubPlan are not yet supported".to_string(),
            input: format!("Expression is {expr:?}"),
            msg_span: span,
            input_span: Span::unknown(),
        }),
        // the parameter polars_plan::dsl::selector::Selector is not publicly exposed.
        // I am not sure what we can meaningfully do with this at this time.
        Expr::Selector(_) => Err(ShellError::UnsupportedInput {
            msg: "Expressions of type Selector to Nu Value is not yet supported".to_string(),
            input: format!("Expression is {expr:?}"),
            msg_span: span,
            input_span: Span::unknown(),
        }),
        Expr::Eval { .. } => Err(ShellError::UnsupportedInput {
            msg: "Expressions of type Eval to Nu Value is not yet supported".to_string(),
            input: format!("Expression is {expr:?}"),
            msg_span: span,
            input_span: Span::unknown(),
        }),
        Expr::Field(column_name) => {
            let fields: Vec<Value> = column_name
                .iter()
                .map(|s| Value::string(s.to_string(), span))
                .collect();
            Ok(Value::record(
                record!(
                    "fields" => Value::list(fields, span)
                ),
                span,
            ))
        }
        Expr::DataTypeFunction(func) => Ok(Value::record(
            record! {
                "expr" => Value::string("data_type_function", span),
                "value" => Value::string(format!("{func:?}"), span),
            },
            span,
        )),
    }
}

impl Cacheable for NuExpression {
    fn cache_id(&self) -> &Uuid {
        &self.id
    }

    fn to_cache_value(&self) -> Result<PolarsPluginObject, ShellError> {
        Ok(PolarsPluginObject::NuExpression(self.clone()))
    }

    fn from_cache_value(cv: PolarsPluginObject) -> Result<Self, ShellError> {
        match cv {
            PolarsPluginObject::NuExpression(df) => Ok(df),
            _ => Err(ShellError::GenericError {
                error: "Cache value is not an expression".into(),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            }),
        }
    }
}

impl CustomValueSupport for NuExpression {
    type CV = NuExpressionCustomValue;

    fn custom_value(self) -> Self::CV {
        NuExpressionCustomValue {
            id: self.id,
            expr: Some(self),
        }
    }

    fn get_type_static() -> PolarsPluginType {
        PolarsPluginType::NuExpression
    }

    fn try_from_value(plugin: &PolarsPlugin, value: &Value) -> Result<Self, ShellError> {
        match value {
            Value::Custom { val, .. } => {
                if let Some(cv) = val.as_any().downcast_ref::<Self::CV>() {
                    Self::try_from_custom_value(plugin, cv)
                } else {
                    Err(ShellError::CantConvert {
                        to_type: Self::get_type_static().to_string(),
                        from_type: value.get_type().to_string(),
                        span: value.span(),
                        help: None,
                    })
                }
            }
            Value::String { val, .. } => Ok(val.to_owned().lit().into()),
            Value::Int { val, .. } => Ok(val.to_owned().lit().into()),
            Value::Bool { val, .. } => Ok(val.to_owned().lit().into()),
            Value::Float { val, .. } => Ok(val.to_owned().lit().into()),
            Value::Date { val, .. } => val
                .to_owned()
                .timestamp_nanos_opt()
                .ok_or_else(|| ShellError::GenericError {
                    error: "Integer overflow".into(),
                    msg: "Provided datetime in nanoseconds is too large for i64".into(),
                    span: Some(value.span()),
                    help: None,
                    inner: vec![],
                })
                .map(|nanos| -> NuExpression {
                    nanos
                        .lit()
                        .strict_cast(polars::prelude::DataType::Datetime(
                            polars::prelude::TimeUnit::Nanoseconds,
                            None,
                        ))
                        .into()
                }),
            Value::Duration { val, .. } => Ok(val
                .to_owned()
                .lit()
                .strict_cast(polars::prelude::DataType::Duration(
                    polars::prelude::TimeUnit::Nanoseconds,
                ))
                .into()),
            x => Err(ShellError::CantConvert {
                to_type: "lazy expression".into(),
                from_type: x.get_type().to_string(),
                span: x.span(),
                help: None,
            }),
        }
    }

    fn can_downcast(value: &Value) -> bool {
        match value {
            Value::Custom { val, .. } => val.as_any().downcast_ref::<Self::CV>().is_some(),
            Value::List { vals, .. } => vals.iter().all(Self::can_downcast),
            Value::Record { val, .. } => val.iter().all(|(_, value)| Self::can_downcast(value)),
            Value::String { .. } | Value::Int { .. } | Value::Bool { .. } | Value::Float { .. } => {
                true
            }
            _ => false,
        }
    }

    fn base_value(self, _span: Span) -> Result<Value, ShellError> {
        self.to_value(Span::unknown())
    }
}
