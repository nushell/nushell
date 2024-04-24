use super::NuLazyGroupBy;
use nu_protocol::{record, CustomValue, ShellError, Span, Value};

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

        Value::custom(Box::new(cloned), span)
    }

    fn type_name(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::record(
            record! {
                "LazyGroupBy" => Value::string("apply aggregation to complete execution plan", span)
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
