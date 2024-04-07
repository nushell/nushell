use super::NuLazyFrame;
use nu_protocol::{record, CustomValue, ShellError, Span, Value};

// CustomValue implementation for NuDataFrame
impl CustomValue for NuLazyFrame {
    fn typetag_name(&self) -> &'static str {
        "lazyframe"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }

    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        let cloned = NuLazyFrame {
            lazy: self.lazy.clone(),
            from_eager: self.from_eager,
            schema: self.schema.clone(),
        };

        Value::custom(Box::new(cloned), span)
    }

    fn type_name(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        let optimized_plan = self
            .as_ref()
            .describe_optimized_plan()
            .unwrap_or_else(|_| "<NOT AVAILABLE>".to_string());

        Ok(Value::record(
            record! {
                "plan" => Value::string(self.as_ref().describe_plan(), span),
                "optimized_plan" => Value::string(optimized_plan, span),
            },
            span,
        ))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
