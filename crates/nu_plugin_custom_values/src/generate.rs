use crate::{cool_custom_value::CoolCustomValue, CustomValuePlugin};

use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, SimplePluginCommand};
use nu_protocol::{Category, PluginSignature, Value};

pub struct Generate;

impl SimplePluginCommand for Generate {
    type Plugin = CustomValuePlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("custom-value generate")
            .usage("PluginSignature for a plugin that generates a custom value")
            .category(Category::Experimental)
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
