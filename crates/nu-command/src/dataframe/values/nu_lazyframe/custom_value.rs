use super::NuLazyFrame;
use nu_protocol::{ast::Operator, CustomValue, ShellError, Span, Value};

// CustomValue implementation for NuDataFrame
impl CustomValue for NuLazyFrame {
    fn typetag_name(&self) -> &'static str {
        "lazyframe"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }

    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        let cloned = NuLazyFrame(self.0.clone());

        Value::CustomValue {
            val: Box::new(cloned),
            span,
        }
    }

    fn value_string(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        unimplemented!()
    }

    fn to_json(&self) -> nu_json::Value {
        nu_json::Value::Null
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn follow_path_int(&self, count: usize, span: Span) -> Result<Value, ShellError> {
        unimplemented!()
    }

    fn follow_path_string(&self, column_name: String, span: Span) -> Result<Value, ShellError> {
        unimplemented!()
    }

    fn partial_cmp(&self, other: &Value) -> Option<std::cmp::Ordering> {
        None
    }

    fn operation(
        &self,
        lhs_span: Span,
        operator: Operator,
        op: Span,
        right: &Value,
    ) -> Result<Value, ShellError> {
        unimplemented!()
    }
}
