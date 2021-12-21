use super::NuGroupBy;
use nu_protocol::{CustomValue, ShellError, Span, Value};

// CustomValue implementation for NuDataFrame
impl CustomValue for NuGroupBy {
    fn typetag_name(&self) -> &'static str {
        "groupby"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }

    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        let cloned = NuGroupBy {
            dataframe: self.dataframe.clone(),
            by: self.by.clone(),
            groups: self.groups.clone(),
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
        let vals = self.print(span)?;

        Ok(Value::List { vals, span })
    }

    fn to_json(&self) -> nu_json::Value {
        nu_json::Value::Null
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
