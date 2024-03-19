use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, SimplePluginCommand};
use nu_protocol::{Category, PluginSignature, SyntaxShape, Value};

use crate::Example;

pub struct Three;

impl SimplePluginCommand for Three {
    type Plugin = Example;

    fn signature(&self) -> PluginSignature {
        // The signature defines the usage of the command inside Nu, and also automatically
        // generates its help page.
        PluginSignature::build("example three")
            .usage("PluginSignature test 3 for plugin. Returns labeled error")
            .required("a", SyntaxShape::Int, "required integer value")
            .required("b", SyntaxShape::String, "required string value")
            .switch("flag", "a flag for the signature", Some('f'))
            .optional("opt", SyntaxShape::Int, "Optional number")
            .named("named", SyntaxShape::String, "named string", Some('n'))
            .rest("rest", SyntaxShape::String, "rest value string")
            .category(Category::Experimental)
    }

    fn run(
        &self,
        plugin: &Example,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        plugin.print_values(3, call, input)?;

        Err(LabeledError {
            label: "ERROR from plugin".into(),
            msg: "error message pointing to call head span".into(),
            span: Some(call.head),
        })
    }
}
