use super::PluginCustomValue;
use crate::plugin::PluginSource;
use nu_protocol::{CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TestCustomValue(pub i32);

#[typetag::serde]
impl CustomValue for TestCustomValue {
    fn clone_value(&self, span: Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        "TestCustomValue".into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::int(self.0 as i64, span))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub(crate) fn test_plugin_custom_value() -> PluginCustomValue {
    let data = bincode::serialize(&expected_test_custom_value() as &dyn CustomValue)
        .expect("bincode serialization of the expected_test_custom_value() failed");

    PluginCustomValue::new("TestCustomValue".into(), data, false, None)
}

pub(crate) fn expected_test_custom_value() -> TestCustomValue {
    TestCustomValue(-1)
}

pub(crate) fn test_plugin_custom_value_with_source() -> PluginCustomValue {
    test_plugin_custom_value().with_source(Some(PluginSource::new_fake("test").into()))
}
