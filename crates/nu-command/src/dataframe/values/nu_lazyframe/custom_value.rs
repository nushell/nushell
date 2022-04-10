use super::NuLazyFrame;
use nu_protocol::{CustomValue, ShellError, Span, Value};

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
        // TODO. Better representation of the lazy frame in nushell
        Ok(Value::nothing(span))
    }

    fn to_json(&self) -> nu_json::Value {
        nu_json::Value::Null
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
