use crate::{update::Update, CustomValuePlugin};

use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, SimplePluginCommand};
use nu_protocol::{Category, PluginSignature, SyntaxShape, Value};

pub struct UpdateArg;

impl SimplePluginCommand for UpdateArg {
    type Plugin = CustomValuePlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("custom-value update-arg")
            .usage("PluginSignature for a plugin that updates a custom value as an argument")
            .required(
                "custom_value",
                SyntaxShape::Any,
                "the custom value to update",
            )
            .category(Category::Experimental)
    }

    fn run(
        &self,
        plugin: &CustomValuePlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        SimplePluginCommand::run(&Update, plugin, engine, call, &call.req(0)?)
    }
}
