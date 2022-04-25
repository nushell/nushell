use nu_protocol::{CustomValue, PipelineData, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use sqlparser::ast::SelectItem;

#[derive(Debug, Serialize, Deserialize)]
pub struct SelectDb(SelectItem);

// Referenced access to the native expression
impl AsRef<SelectItem> for SelectDb {
    fn as_ref(&self) -> &SelectItem {
        &self.0
    }
}

impl AsMut<SelectItem> for SelectDb {
    fn as_mut(&mut self) -> &mut SelectItem {
        &mut self.0
    }
}

impl From<SelectItem> for SelectDb {
    fn from(expr: SelectItem) -> Self {
        Self(expr)
    }
}

impl SelectDb {
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

    pub fn into_native(self) -> SelectItem {
        self.0
    }

    pub fn to_value(&self, span: Span) -> Value {
        select_to_value(self.as_ref(), span)
    }
}

impl CustomValue for SelectDb {
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
        "DB selection"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }
}

fn select_to_value(select: &SelectItem, span: Span) -> Value {
    match select {
        SelectItem::UnnamedExpr(_) => Value::String{ val: "unnamed".into(), span },
        SelectItem::ExprWithAlias { .. } => Value::String{ val: "with_alias".into(), span },
        SelectItem::QualifiedWildcard(_) => Value::String{ val: "qualified_wildcard".into(), span },
        SelectItem::Wildcard => Value::String{ val: "wildcard".into(), span },
    }
}
