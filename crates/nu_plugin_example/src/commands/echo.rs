use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, Type, Value};

use crate::ExamplePlugin;

/// `<list> | example echo`
pub struct Echo;

impl PluginCommand for Echo {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example echo"
    }

    fn description(&self) -> &str {
        "Example stream consumer that outputs the received input"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Any, Type::Any)])
            .category(Category::Experimental)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["example"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "example seq 1 5 | example echo",
            description: "echos the values from 1 to 5",
            result: Some(Value::test_list(
                (1..=5).map(Value::test_int).collect::<Vec<_>>(),
            )),
        }]
    }

    fn run(
        &self,
        _plugin: &ExamplePlugin,
        _engine: &EngineInterface,
        _call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        Ok(input)
    }
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;
    PluginTest::new("example", ExamplePlugin.into())?.test_command_examples(&Echo)
}
