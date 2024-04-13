use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, LabeledError, Signature, Value};

use crate::ExamplePlugin;

pub struct Main;

impl SimplePluginCommand for Main {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example"
    }

    fn usage(&self) -> &str {
        "Example commands for Nushell plugins"
    }

    fn extra_usage(&self) -> &str {
        r#"
The `example` plugin demonstrates usage of the Nushell plugin API.

Several commands provided to test and demonstrate different capabilities of
plugins exposed through the API. None of these commands are intended to be
particularly useful.
"#
        .trim()
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Experimental)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["example"]
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        Ok(Value::string(engine.get_help()?, call.head))
    }
}
