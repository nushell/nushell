use crate::{cool_custom_value::CoolCustomValue, CustomValuePlugin};
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, Example, LabeledError, Signature, Span, Value};

pub struct Generate;

impl SimplePluginCommand for Generate {
    type Plugin = CustomValuePlugin;

    fn name(&self) -> &str {
        "custom-value generate"
    }

    fn usage(&self) -> &str {
        "PluginSignature for a plugin that generates a custom value"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "custom-value generate",
            description: "Generate a new CoolCustomValue",
            result: Some(CoolCustomValue::new("abc").into_value(Span::test_data())),
        }]
    }

    fn run(
        &self,
        _plugin: &CustomValuePlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        Ok(CoolCustomValue::new("abc").into_value(call.head))
    }
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;

    PluginTest::new("custom_values", crate::CustomValuePlugin.into())?
        .test_command_examples(&Generate)
}
