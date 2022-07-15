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
        let cloned = NuLazyFrame {
            lazy: self.lazy.clone(),
            from_eager: self.from_eager,
            schema: self.schema.clone(),
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
        let cols = vec!["plan".into(), "optimized_plan".into()];
        let vals = vec![
            Value::String {
                val: self.as_ref().describe_plan(),
                span,
            },
            Value::String {
                val: self
                    .as_ref()
                    .describe_optimized_plan()
                    .unwrap_or_else(|_| "<NOT AVAILABLE>".to_string()),
                span,
            },
        ];

        Ok(Value::Record { cols, vals, span })
    }

    fn to_json(&self) -> nu_json::Value {
        nu_json::Value::Null
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
