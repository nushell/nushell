use crate::PluginCustomValue;
use nu_protocol::{CustomValue, FutureSpanId, ShellError, Value};
use serde::{Deserialize, Serialize};

/// A custom value that can be used for testing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestCustomValue(pub i32);

#[typetag::serde(name = "nu_plugin_protocol::test_util::TestCustomValue")]
impl CustomValue for TestCustomValue {
    fn clone_value(&self, span: FutureSpanId) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        "TestCustomValue".into()
    }

    fn to_base_value(&self, span: FutureSpanId) -> Result<Value, ShellError> {
        Ok(Value::int(self.0 as i64, span))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A [`TestCustomValue`] serialized as a [`PluginCustomValue`].
pub fn test_plugin_custom_value() -> PluginCustomValue {
    let data = bincode::serialize(&expected_test_custom_value() as &dyn CustomValue)
        .expect("bincode serialization of the expected_test_custom_value() failed");

    PluginCustomValue::new("TestCustomValue".into(), data, false)
}

/// The expected [`TestCustomValue`] that [`test_plugin_custom_value()`] should deserialize into.
pub fn expected_test_custom_value() -> TestCustomValue {
    TestCustomValue(-1)
}
