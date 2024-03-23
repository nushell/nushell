use nu_plugin::{Plugin, SimplePluginCommand};
use nu_plugin_test_support::PluginTest;
use nu_protocol::{
    CustomValue, LabeledError, PluginExample, PluginSignature, ShellError, Span, Type, Value,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CustomU32(u32);

impl CustomU32 {
    pub fn into_value(self, span: Span) -> Value {
        Value::custom_value(Box::new(self), span)
    }
}

#[typetag::serde]
impl CustomValue for CustomU32 {
    fn clone_value(&self, span: Span) -> Value {
        self.clone().into_value(span)
    }

    fn type_name(&self) -> String {
        "CustomU32".into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::int(self.0 as i64, span))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct CustomU32Plugin;
struct IntoU32;

impl Plugin for CustomU32Plugin {
    fn commands(&self) -> Vec<Box<dyn nu_plugin::PluginCommand<Plugin = Self>>> {
        vec![Box::new(IntoU32)]
    }
}

impl SimplePluginCommand for IntoU32 {
    type Plugin = CustomU32Plugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("into u32")
            .input_output_type(Type::Int, Type::Custom("CustomU32".into()))
            .plugin_examples(vec![PluginExample {
                example: "340 | into u32".into(),
                description: "Make a u32".into(),
                result: Some(CustomU32(340).into_value(Span::test_data())),
            }])
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let value: i64 = input.as_int()?;
        let value_u32 = u32::try_from(value).map_err(|err| {
            LabeledError::new(format!("Not a valid u32: {value}"))
                .with_label(err.to_string(), input.span())
        })?;
        Ok(CustomU32(value_u32).into_value(call.head))
    }
}

#[test]
fn test_into_u32_examples() -> Result<(), ShellError> {
    PluginTest::new("custom_u32", CustomU32Plugin.into())?.test_command_examples(&IntoU32)
}
