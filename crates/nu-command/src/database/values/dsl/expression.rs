use nu_protocol::{CustomValue, PipelineData, ShellError, Span, Value};
use serde::{Deserialize, Serialize};

use sqlparser::ast::{Expr, Ident};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExprDb(Expr);

// Referenced access to the native expression
impl AsRef<Expr> for ExprDb {
    fn as_ref(&self) -> &Expr {
        &self.0
    }
}

impl AsMut<Expr> for ExprDb {
    fn as_mut(&mut self) -> &mut Expr {
        &mut self.0
    }
}

impl From<Expr> for ExprDb {
    fn from(expr: Expr) -> Self {
        Self(expr)
    }
}

impl CustomValue for ExprDb {
    fn clone_value(&self, span: Span) -> Value {
        let cloned = Self(self.0.clone());

        Value::CustomValue {
            val: Box::new(cloned),
            span,
        }
    }

    fn value_string(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(self.to_value(span))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn typetag_name(&self) -> &'static str {
        "DB expresssion"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }
    
    fn operation(
        &self,
        _lhs_span: Span,
        operator: Operator,
        op: Span,
        _right: &Value,
    ) -> Result<Value, ShellError> {
        Err(ShellError::UnsupportedOperator(operator, op))
    }
}

impl ExprDb {
    pub fn try_from_value(value: Value) -> Result<Self, ShellError> {
        match value {
            Value::CustomValue { val, span } => match val.as_any().downcast_ref::<Self>() {
                Some(expr) => Ok(Self(expr.0.clone())),
                None => Err(ShellError::CantConvert(
                    "db expression".into(),
                    "non-expression".into(),
                    span,
                    None,
                )),
            },
            Value::String { val, .. } => Ok(Expr::Identifier(Ident {
                value: val,
                quote_style: None,
            })
            .into()),
            x => Err(ShellError::CantConvert(
                "database".into(),
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

    pub fn into_value(self, span: Span) -> Value {
        Value::CustomValue {
            val: Box::new(self),
            span,
        }
    }

    pub fn into_native(self) -> Expr {
        self.0
    }

    pub fn to_value(&self, span: Span) -> Value {
        ExprDb::expr_to_value(self.as_ref(), span)
    }
}

impl ExprDb {
    pub fn expr_to_value(expr: &Expr, span: Span) -> Value {
        match expr {
            Expr::Identifier(ident) => {
                let cols = vec!["value".into(), "quoted_style".into()];
                let val = Value::String {
                    val: ident.value.to_string(),
                    span,
                };
                let style = Value::String {
                    val: format!("{:?}", ident.quote_style),
                    span,
                };

                Value::Record {
                    cols,
                    vals: vec![val, style],
                    span,
                }
            }
            Expr::CompoundIdentifier(_) => todo!(),
            Expr::IsNull(_) => todo!(),
            Expr::IsNotNull(_) => todo!(),
            Expr::IsDistinctFrom(_, _) => todo!(),
            Expr::IsNotDistinctFrom(_, _) => todo!(),
            Expr::InList { .. } => todo!(),
            Expr::InSubquery { .. } => todo!(),
            Expr::InUnnest { .. } => todo!(),
            Expr::Between { .. } => todo!(),
            Expr::BinaryOp { .. } => todo!(),
            Expr::UnaryOp { .. } => todo!(),
            Expr::Cast { .. } => todo!(),
            Expr::TryCast { .. } => todo!(),
            Expr::Extract { .. } => todo!(),
            Expr::Substring { .. } => todo!(),
            Expr::Trim { .. } => todo!(),
            Expr::Collate { .. } => todo!(),
            Expr::Nested(_) => todo!(),
            Expr::Value(_) => todo!(),
            Expr::TypedString { .. } => todo!(),
            Expr::MapAccess { .. } => todo!(),
            Expr::Function(_) => todo!(),
            Expr::Case { .. } => todo!(),
            Expr::Exists(_) => todo!(),
            Expr::Subquery(_) => todo!(),
            Expr::ListAgg(_) => todo!(),
            Expr::GroupingSets(_) => todo!(),
            Expr::Cube(_) => todo!(),
            Expr::Rollup(_) => todo!(),
            Expr::Tuple(_) => todo!(),
            Expr::ArrayIndex { .. } => todo!(),
            Expr::Array(_) => todo!(),
        }
    }
}
