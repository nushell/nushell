use crate::{CustomValuePlugin, second_custom_value::SecondCustomValue};
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, Example, LabeledError, Signature, Span, SyntaxShape, Value};

pub struct Generate2;

impl SimplePluginCommand for Generate2 {
    type Plugin = CustomValuePlugin;

    fn name(&self) -> &str {
        "custom-value generate2"
    }

    fn description(&self) -> &str {
        "PluginSignature for a plugin that generates a different custom value"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "An optional closure to pass the custom value to",
            )
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "custom-value generate2",
                description: "Generate a new SecondCustomValue",
                result: Some(SecondCustomValue::new("xyz").into_value(Span::test_data())),
            },
            Example {
                example: "custom-value generate2 { print }",
                description: "Generate a new SecondCustomValue and pass it to a closure",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        _plugin: &CustomValuePlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        let second_custom_value = SecondCustomValue::new("xyz").into_value(call.head);
        // If we were passed a closure, execute that instead
        if let Some(closure) = call.opt(0)? {
            let result = engine.eval_closure(
                &closure,
                vec![second_custom_value.clone()],
                Some(second_custom_value),
            )?;
            Ok(result)
        } else {
            Ok(second_custom_value)
        }
    }
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;

    PluginTest::new("custom_values", crate::CustomValuePlugin::new().into())?
        .test_command_examples(&Generate2)
}
