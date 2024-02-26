use nu_protocol::{CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};

use crate::plugin::PluginIdentity;

use super::PluginCustomValue;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TestCustomValue(pub i32);

#[typetag::serde]
impl CustomValue for TestCustomValue {
    fn clone_value(&self, span: Span) -> Value {
        Value::custom_value(Box::new(self.clone()), span)
    }

    fn value_string(&self) -> String {
        "TestCustomValue".into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::int(self.0 as i64, span))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub(crate) fn test_plugin_custom_value() -> PluginCustomValue {
    let data = bincode::serialize(&expected_test_custom_value() as &dyn CustomValue)
        .expect("bincode serialization of the expected_test_custom_value() failed");

    PluginCustomValue {
        name: "TestCustomValue".into(),
        data,
        source: None,
    }
}

pub(crate) fn expected_test_custom_value() -> TestCustomValue {
    TestCustomValue(-1)
}

pub(crate) fn test_plugin_custom_value_with_source() -> PluginCustomValue {
    PluginCustomValue {
        source: Some(PluginIdentity::new_fake("test")),
        ..test_plugin_custom_value()
    }
}
