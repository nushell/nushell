use crate::{second_custom_value::SecondCustomValue, CustomValuePlugin};

use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, SimplePluginCommand};
use nu_protocol::{Category, PluginSignature, SyntaxShape, Value};

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
