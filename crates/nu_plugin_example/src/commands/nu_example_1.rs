use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, SimplePluginCommand};
use nu_protocol::{Category, PluginExample, PluginSignature, SyntaxShape, Value};

use crate::Example;

pub struct NuExample1;

impl SimplePluginCommand for NuExample1 {
    type Plugin = Example;

    fn signature(&self) -> PluginSignature {
        // The signature defines the usage of the command inside Nu, and also automatically
        // generates its help page.
        PluginSignature::build("nu-example-1")
            .usage("PluginSignature test 1 for plugin. Returns Value::Nothing")
            .extra_usage("Extra usage for nu-example-1")
            .search_terms(vec!["example".into()])
            .required("a", SyntaxShape::Int, "required integer value")
            .required("b", SyntaxShape::String, "required string value")
            .switch("flag", "a flag for the signature", Some('f'))
            .optional("opt", SyntaxShape::Int, "Optional number")
            .named("named", SyntaxShape::String, "named string", Some('n'))
            .rest("rest", SyntaxShape::String, "rest value string")
            .plugin_examples(vec![PluginExample {
                example: "nu-example-1 3 bb".into(),
                description: "running example with an int value and string value".into(),
                result: None,
            }])
            .category(Category::Experimental)
    }

    fn run(
        &self,
        plugin: &Example,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        plugin.print_values(1, call, input)?;

        Ok(Value::nothing(call.head))
    }
}
