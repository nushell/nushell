mod custom_value;

use core::fmt;
use nu_protocol::{PipelineData, ShellError, Span, Value};
use polars::prelude::{Expr, Literal};
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

impl NuExpression {
    pub fn new(expr: Expr) -> Self {
        Self(Some(expr))
    }

    pub fn into_value(self, span: Span) -> Value {
        Value::CustomValue {
            val: Box::new(self),
            span,
        }
    }

    pub fn try_from_value(value: Value) -> Result<Self, ShellError> {
        match value {
            Value::CustomValue { val, span } => match val.as_any().downcast_ref::<NuExpression>() {
                Some(expr) => Ok(NuExpression(expr.0.clone())),
                None => Err(ShellError::CantConvert(
                    "lazy expression".into(),
                    "non-dataframe".into(),
                    span,
                )),
            },
            Value::String { val, .. } => Ok(Self::new(val.lit())),
            Value::Int { val, .. } => Ok(Self::new(val.lit())),
            Value::Bool { val, .. } => Ok(Self::new(val.lit())),
            Value::Float { val, .. } => Ok(Self::new(val.lit())),
            x => Err(ShellError::CantConvert(
                "lazy expression".into(),
                x.get_type().to_string(),
                x.span()?,
            )),
        }
    }

    pub fn try_from_pipeline(input: PipelineData, span: Span) -> Result<Self, ShellError> {
        let value = input.into_value(span);
        Self::try_from_value(value)
    }

    pub fn into_polars(self) -> Expr {
        self.0.expect("Expression cannot be none to convert")
    }

    //pub fn apply<F>(self, f: F) -> Self
    //where
    //    F: Fn(Expr) -> Expr,
    //{
    //    let expr = self.0.expect("Lazy expression must not be empty to apply");
    //    let new_expr = f(expr);

    //    Self::new(new_expr)
    //}

    pub fn apply_with_expr<F>(self, other: NuExpression, f: F) -> Self
    where
        F: Fn(Expr, Expr) -> Expr,
    {
        let expr = self.0.expect("Lazy expression must not be empty to apply");
        let other = other.0.expect("Lazy expression must not be empty to apply");
        let new_expr = f(expr, other);

        Self::new(new_expr)
    }

    pub fn to_value(&self, span: Span) -> Value {
        expr_to_value(self.as_ref(), span)
    }
}

pub fn expr_to_value(expr: &Expr, span: Span) -> Value {
    let cols = vec!["expr".to_string(), "value".to_string()];

    println!("{:?}", expr);
    match expr {
        Expr::Not(_) => todo!(),
        Expr::Alias(..) => todo!(),
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
            let left_val = expr_to_value(&left, span.clone());
            let right_val = expr_to_value(&right, span.clone());

            let operator = Value::String {
                val: format!("{:?}", op),
                span: span.clone(),
            };

            let cols = vec!["left".to_string(), "op".to_string(), "right".to_string()];

            Value::Record {
                cols,
                vals: vec![left_val, operator, right_val],
                span,
            }
        }
        Expr::IsNotNull(_) => todo!(),
        Expr::IsNull(_) => todo!(),
        Expr::Cast { .. } => todo!(),
        Expr::Sort { .. } => todo!(),
        Expr::Take { .. } => todo!(),
        Expr::SortBy { .. } => todo!(),
        Expr::Agg(_) => todo!(),
        Expr::Ternary { .. } => todo!(),
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
    }
}
