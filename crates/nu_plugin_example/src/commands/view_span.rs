use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, Example, LabeledError, Signature, Type, Value};

use crate::ExamplePlugin;

/// `<value> | example view span`
pub struct ViewSpan;

impl SimplePluginCommand for ViewSpan {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example view span"
    }

    fn description(&self) -> &str {
        "Example command for looking up the contents of a parser span"
    }

    fn extra_description(&self) -> &str {
        "Shows the original source code of the expression that generated the value passed as input."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Any, Type::String)
            .category(Category::Experimental)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["example"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "('hello ' ++ 'world') | example view span",
            description: "Show the source code of the expression that generated a value",
            result: Some(Value::test_string("'hello ' ++ 'world'")),
        }]
    }

    fn run(
        &self,
        _plugin: &ExamplePlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let contents = engine.get_span_contents(input.span())?;
        Ok(Value::string(String::from_utf8_lossy(&contents), call.head))
    }
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;
    PluginTest::new("example", ExamplePlugin.into())?.test_command_examples(&ViewSpan)
}
