use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, SimplePluginCommand};
use nu_protocol::{Category, PluginSignature, Value};

use crate::Example;

pub struct Main;

impl SimplePluginCommand for Main {
    type Plugin = Example;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("example")
            .usage("Example commands for Nushell plugins")
            .extra_usage(
                r#"
The `example` plugin demonstrates usage of the Nushell plugin API.

Several commands provided to test and demonstrate different capabilities of
plugins exposed through the API. None of these commands are intended to be
particularly useful.
"#
                .trim(),
            )
            .category(Category::Experimental)
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        Err(LabeledError {
            label: "No subcommand provided".into(),
            msg: "add --help to see a list of subcommands".into(),
            span: Some(call.head.past()),
        })
    }
}
