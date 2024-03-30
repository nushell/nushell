use super::NuDataFrame;
use nu_protocol::{ast::Operator, CustomValue, ShellError, Span, SpanId, Value};

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

    fn to_base_value(&self, span: Span, span_id: SpanId) -> Result<Value, ShellError> {
        let vals = self.print(span, span_id)?;

        Ok(Value::list(vals, span))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn follow_path_int(
        &self,
        _self_span: Span,
        _self_span_id: SpanId,
        count: usize,
        path_span: Span,
        path_span_id: SpanId,
    ) -> Result<Value, ShellError> {
        self.get_value(count, path_span, path_span_id)
    }

    fn follow_path_string(
        &self,
        _self_span: Span,
        _self_span_id: SpanId,
        column_name: String,
        path_span: Span,
        path_span_id: SpanId,
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
        lhs_span_id: SpanId,
        operator: Operator,
        op: Span,
        op_id: SpanId,
        right: &Value,
    ) -> Result<Value, ShellError> {
        self.compute_with_value(lhs_span, operator, op, op_id, right)
    }
}
