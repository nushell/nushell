use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, LabeledError, PluginExample, PluginSignature, SyntaxShape, Value};

use crate::Example;

pub struct One;

impl SimplePluginCommand for One {
    type Plugin = Example;

    fn signature(&self) -> PluginSignature {
        // The signature defines the usage of the command inside Nu, and also automatically
        // generates its help page.
        PluginSignature::build("example one")
            .usage("PluginSignature test 1 for plugin. Returns Value::Nothing")
            .extra_usage("Extra usage for example one")
            .search_terms(vec!["example".into()])
            .required("a", SyntaxShape::Int, "required integer value")
            .required("b", SyntaxShape::String, "required string value")
            .switch("flag", "a flag for the signature", Some('f'))
            .optional("opt", SyntaxShape::Int, "Optional number")
            .named("named", SyntaxShape::String, "named string", Some('n'))
            .rest("rest", SyntaxShape::String, "rest value string")
            .plugin_examples(vec![PluginExample {
                example: "example one 3 bb".into(),
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

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;
    PluginTest::new("example", Example.into())?.test_command_examples(&One)
}
