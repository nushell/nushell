use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    ByteStream, ByteStreamType, Category, Example, LabeledError, PipelineData, Signals, Signature,
    Type, Value,
};

use crate::ExamplePlugin;

/// `<list<string>> | example collect-bytes`
pub struct CollectBytes;

impl PluginCommand for CollectBytes {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example collect-bytes"
    }

    fn description(&self) -> &str {
        "Example transformer to byte stream"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["example"]
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::List(Type::String.into()), Type::String),
                (Type::List(Type::Binary.into()), Type::Binary),
            ])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "[a b] | example collect-bytes",
            description: "collect strings into one stream",
            result: Some(Value::test_string("ab")),
        }]
    }

    fn run(
        &self,
        _plugin: &ExamplePlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        Ok(PipelineData::byte_stream(
            ByteStream::from_result_iter(
                input.into_iter().map(Value::coerce_into_binary),
                call.head,
                Signals::empty(),
                ByteStreamType::Unknown,
            ),
            None,
        ))
    }
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;
    PluginTest::new("example", ExamplePlugin.into())?.test_command_examples(&CollectBytes)
}
