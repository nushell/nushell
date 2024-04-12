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

        Value::custom(Box::new(cloned), span)
    }

    fn type_name(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        let vals = self.print(span)?;

        Ok(Value::list(vals, span))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn follow_path_int(
        &self,
        _self_span: Span,
        count: usize,
        path_span: Span,
    ) -> Result<Value, ShellError> {
        self.get_value(count, path_span)
    }

    fn follow_path_string(
        &self,
        _self_span: Span,
        column_name: String,
        path_span: Span,
    ) -> Result<Value, ShellError> {
        let column = self.column(&column_name, path_span)?;
        Ok(column.into_value(path_span))
    }

    fn partial_cmp(&self, other: &Value) -> Option<std::cmp::Ordering> {
        match other {
            Value::Custom { val, .. } => val
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
