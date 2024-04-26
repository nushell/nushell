use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, LabeledError, Signature, Type, Value};

use crate::ExamplePlugin;

pub struct Config;

impl SimplePluginCommand for Config {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example config"
    }

    fn usage(&self) -> &str {
        "Show plugin configuration"
    }

    fn extra_usage(&self) -> &str {
        "The configuration is set under $env.config.plugins.example"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Experimental)
            .input_output_type(Type::Nothing, Type::table())
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["example", "configuration"]
    }

    fn run(
        &self,
        _plugin: &ExamplePlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        let config = engine.get_plugin_config()?;
        match config {
            Some(config) => Ok(config.clone()),
            None => Err(LabeledError::new("No config sent").with_label(
                "configuration for this plugin was not found in `$env.config.plugins.example`",
                call.head,
            )),
        }
    }
}
