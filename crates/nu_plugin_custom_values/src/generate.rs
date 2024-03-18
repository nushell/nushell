use crate::{cool_custom_value::CoolCustomValue, CustomValuePlugin};

use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, SimplePluginCommand};
use nu_protocol::{Category, PluginExample, PluginSignature, Span, Value};

pub struct Generate;

impl SimplePluginCommand for Generate {
    type Plugin = CustomValuePlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("custom-value generate")
            .usage("PluginSignature for a plugin that generates a custom value")
            .category(Category::Experimental)
            .plugin_examples(vec![PluginExample {
                example: "custom-value generate".into(),
                description: "Generate a new CoolCustomValue".into(),
                result: Some(CoolCustomValue::new("abc").into_value(Span::test_data())),
            }])
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
