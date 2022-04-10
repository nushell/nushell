use super::NuExpression;
use nu_protocol::{CustomValue, ShellError, Span, Value};

// CustomValue implementation for NuDataFrame
impl CustomValue for NuExpression {
    fn typetag_name(&self) -> &'static str {
        "expression"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }

    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        let cloned = NuExpression(self.0.clone());

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

    fn to_json(&self) -> nu_json::Value {
        nu_json::Value::Null
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
