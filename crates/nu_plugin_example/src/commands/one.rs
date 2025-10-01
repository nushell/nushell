use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, Example, LabeledError, Signature, SyntaxShape, Value};

use crate::ExamplePlugin;

pub struct One;

impl SimplePluginCommand for One {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example one"
    }

    fn description(&self) -> &str {
        "Plugin test example 1. Returns Value::Nothing"
    }

    fn extra_description(&self) -> &str {
        "Extra description for example one"
    }

    fn signature(&self) -> Signature {
        // The signature defines the usage of the command inside Nu, and also automatically
        // generates its help page.
        Signature::build(self.name())
            .required("a", SyntaxShape::Int, "required integer value")
            .required("b", SyntaxShape::String, "required string value")
            .switch("flag", "a flag for the signature", Some('f'))
            .optional("opt", SyntaxShape::Int, "Optional number")
            .named("named", SyntaxShape::String, "named string", Some('n'))
            .rest("rest", SyntaxShape::String, "rest value string")
            .category(Category::Experimental)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["example"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "example one 3 bb",
            description: "running example with an int value and string value",
            result: None,
        }]
    }

    fn run(
        &self,
        plugin: &ExamplePlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        plugin.print_values(1, call, input)?;

        Ok(Value::nothing(call.head))
    }
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;
    PluginTest::new("example", ExamplePlugin.into())?.test_command_examples(&One)
}
