use super::NuWhen;
use nu_protocol::{CustomValue, ShellError, Span, Value};

// CustomValue implementation for NuDataFrame
impl CustomValue for NuWhen {
    fn typetag_name(&self) -> &'static str {
        "when"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }

    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        let cloned = self.clone();

        Value::custom(Box::new(cloned), span)
    }

    fn type_name(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        let val: String = match self {
            NuWhen::Then(_) => "whenthen".into(),
            NuWhen::ChainedThen(_) => "whenthenthen".into(),
        };

        let value = Value::string(val, span);
        Ok(value)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
