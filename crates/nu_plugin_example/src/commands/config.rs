use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, SimplePluginCommand};
use nu_protocol::{Category, PluginSignature, Type, Value};

use crate::Example;

pub struct Config;

impl SimplePluginCommand for Config {
    type Plugin = Example;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("example config")
            .usage("Show plugin configuration")
            .extra_usage("The configuration is set under $env.config.plugins.example")
            .category(Category::Experimental)
            .search_terms(vec!["example".into(), "configuration".into()])
            .input_output_type(Type::Nothing, Type::Table(vec![]))
    }

    fn run(
        &self,
        _plugin: &Example,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        let config = engine.get_plugin_config()?;
        match config {
            Some(config) => Ok(config.clone()),
            None => Err(LabeledError {
                label: "No config sent".into(),
                msg: "Configuration for this plugin was not found in `$env.config.plugins.example`"
                    .into(),
                span: Some(call.head),
            }),
        }
    }
}
