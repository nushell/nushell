use super::NuLazyGroupBy;
use nu_protocol::{CustomValue, ShellError, Span, Value};

// CustomValue implementation for NuDataFrame
impl CustomValue for NuLazyGroupBy {
    fn typetag_name(&self) -> &'static str {
        "lazygroupby"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }

    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        let cloned = NuLazyGroupBy {
            group_by: self.group_by.clone(),
            schema: self.schema.clone(),
            from_eager: self.from_eager,
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
        let cols = vec!["LazyGroupBy".into()];
        let vals = vec![Value::String {
            val: "apply aggregation to complete execution plan".into(),
            span,
        }];

        Ok(Value::Record { cols, vals, span })
    }

    fn to_json(&self) -> nu_json::Value {
        nu_json::Value::Null
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
