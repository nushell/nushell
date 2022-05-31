use super::NuDataFrame;
use nu_protocol::{ast::Operator, CustomValue, ShellError, Span, Value};

// CustomValue implementation for NuDataFrame
impl CustomValue for NuDataFrame {
    fn typetag_name(&self) -> &'static str {
        "dataframe"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }

    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        let cloned = NuDataFrame {
            df: self.df.clone(),
            from_lazy: false,
        };

        Value::CustomValue {
            val: Box::new(cloned),
            span,
        }
    }

    fn value_string(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        let vals = self.print(span)?;

        Ok(Value::List { vals, span })
    }

    fn to_json(&self) -> nu_json::Value {
        nu_json::Value::Null
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn follow_path_int(&self, count: usize, span: Span) -> Result<Value, ShellError> {
        self.get_value(count, span)
    }

    fn follow_path_string(&self, column_name: String, span: Span) -> Result<Value, ShellError> {
        let column = self.column(&column_name, span)?;
        Ok(column.into_value(span))
    }

    fn partial_cmp(&self, other: &Value) -> Option<std::cmp::Ordering> {
        match other {
            Value::CustomValue { val, .. } => val
                .as_any()
                .downcast_ref::<Self>()
                .and_then(|other| self.is_equal(other)),
            _ => None,
        }
    }

    fn operation(
        &self,
        lhs_span: Span,
        operator: Operator,
        op: Span,
        right: &Value,
    ) -> Result<Value, ShellError> {
        self.compute_with_value(lhs_span, operator, op, right)
    }
}
