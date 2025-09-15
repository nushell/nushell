use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, ListStream, PipelineData, Signals, Signature, SyntaxShape,
    Type, Value,
};

use crate::ExamplePlugin;

/// `example seq <first> <last>`
pub struct Seq;

impl PluginCommand for Seq {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example seq"
    }

    fn description(&self) -> &str {
        "Example stream generator for a list of values"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("first", SyntaxShape::Int, "first number to generate")
            .required("last", SyntaxShape::Int, "last number to generate")
            .input_output_type(Type::Nothing, Type::List(Type::Int.into()))
            .category(Category::Experimental)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["example"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "example seq 1 3",
            description: "generate a sequence from 1 to 3",
            result: Some(Value::test_list(vec![
                Value::test_int(1),
                Value::test_int(2),
                Value::test_int(3),
            ])),
        }]
    }

    fn run(
        &self,
        _plugin: &ExamplePlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let head = call.head;
        let first: i64 = call.req(0)?;
        let last: i64 = call.req(1)?;
        let iter = (first..=last).map(move |number| Value::int(number, head));
        Ok(ListStream::new(iter, head, Signals::empty()).into())
    }
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;
    PluginTest::new("example", ExamplePlugin.into())?.test_command_examples(&Seq)
}
