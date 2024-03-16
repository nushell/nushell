use crate::{second_custom_value::SecondCustomValue, CustomValuePlugin};
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{
    Category, LabeledError, PluginExample, PluginSignature, Span, SyntaxShape, Value,
};

pub struct Generate2;

impl SimplePluginCommand for Generate2 {
    type Plugin = CustomValuePlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("custom-value generate2")
            .usage("PluginSignature for a plugin that generates a different custom value")
            .optional(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "An optional closure to pass the custom value to",
            )
            .category(Category::Experimental)
            .plugin_examples(vec![
                PluginExample {
                    example: "custom-value generate2".into(),
                    description: "Generate a new SecondCustomValue".into(),
                    result: Some(SecondCustomValue::new("xyz").into_value(Span::test_data())),
                },
                PluginExample {
                    example: "custom-value generate2 { print }".into(),
                    description: "Generate a new SecondCustomValue and pass it to a closure".into(),
                    result: None,
                },
            ])
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

    PluginTest::new("custom_values", crate::CustomValuePlugin.into())?
        .test_command_examples(&Generate2)
}
