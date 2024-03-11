use crate::{
    cool_custom_value::CoolCustomValue, second_custom_value::SecondCustomValue, CustomValuePlugin,
};

use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, SimplePluginCommand};
use nu_protocol::{Category, PluginSignature, ShellError, Value};

pub struct Update;

impl SimplePluginCommand for Update {
    type Plugin = CustomValuePlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("custom-value update")
            .usage("PluginSignature for a plugin that updates a custom value")
            .category(Category::Experimental)
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
