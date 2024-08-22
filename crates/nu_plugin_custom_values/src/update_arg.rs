use crate::{update::Update, CustomValuePlugin};
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, LabeledError, Signature, SyntaxShape, Value};

pub struct UpdateArg;

impl SimplePluginCommand for UpdateArg {
    type Plugin = CustomValuePlugin;

    fn name(&self) -> &str {
        "custom-value update-arg"
    }

    fn description(&self) -> &str {
        "Updates a custom value as an argument"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
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
