use std::path::PathBuf;

use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, FromValue, LabeledError, Signature, Spanned, Type, Value};

use crate::ExamplePlugin;

pub struct Config;

/// Example config struct.
///
/// Using the `FromValue` derive macro, structs can be easily loaded from [`Value`]s,
/// similar to serde's `Deserialize` macro.
/// This is handy for plugin configs or piped data.
/// All fields must implement [`FromValue`].
/// For [`Option`] fields, they can be omitted in the config.
///
/// This example shows that nested and spanned data work too, so you can describe nested
/// structures and get spans of values wrapped in [`Spanned`].
/// Since this config uses only `Option`s, no field is required in the config.
#[allow(dead_code)]
#[derive(Debug, FromValue)]
struct PluginConfig {
    path: Option<Spanned<PathBuf>>,
    nested: Option<SubConfig>,
}

#[allow(dead_code)]
#[derive(Debug, FromValue)]
struct SubConfig {
    bool: bool,
    string: String,
}

impl SimplePluginCommand for Config {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example config"
    }

    fn description(&self) -> &str {
        "Show plugin configuration"
    }

    fn extra_description(&self) -> &str {
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
            Some(value) => {
                let config = PluginConfig::from_value(value.clone())?;
                println!("got config {config:?}");
                Ok(value)
            }
            None => Err(LabeledError::new("No config sent").with_label(
                "configuration for this plugin was not found in `$env.config.plugins.example`",
                call.head,
            )),
        }
    }
}
