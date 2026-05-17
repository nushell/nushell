use crate::{
    CustomValuePlugin, cool_custom_value::CoolCustomValue, second_custom_value::SecondCustomValue,
};
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, Example, LabeledError, ShellError, Signature, Span, Value};

pub struct Update;

impl SimplePluginCommand for Update {
    type Plugin = CustomValuePlugin;

    fn name(&self) -> &str {
        "custom-value update"
    }

    fn description(&self) -> &str {
        "PluginSignature for a plugin that updates a custom value"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "custom-value generate | custom-value update",
                description: "Update a CoolCustomValue",
                result: Some(CoolCustomValue::new("abcxyz").into_value(Span::test_data())),
            },
            Example {
                example: "custom-value generate2 | custom-value update",
                description: "Update a SecondCustomValue",
                result: Some(SecondCustomValue::new("xyzabc").into_value(Span::test_data())),
            },
        ]
    }

    fn run(
        &self,
        _plugin: &CustomValuePlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        if let Ok(mut value) = CoolCustomValue::try_from_value(input) {
            value.cool += "xyz";
            return Ok(value.into_value(call.head));
        }

        if let Ok(mut value) = SecondCustomValue::try_from_value(input) {
            value.something += "abc";
            return Ok(value.into_value(call.head));
        }

        Err(ShellError::CantConvert {
            to_type: "cool or second".into(),
            from_type: "non-cool and non-second".into(),
            span: call.head,
            help: None,
        }
        .into())
    }
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;

    PluginTest::new("custom_values", crate::CustomValuePlugin::new().into())?
        .test_command_examples(&Update)
}
