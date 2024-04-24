use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, RawStream, Signature, Type, Value,
};

use crate::ExamplePlugin;

/// `<list<string>> | example collect-external`
pub struct CollectExternal;

impl PluginCommand for CollectExternal {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example collect-external"
    }

    fn usage(&self) -> &str {
        "Example transformer to raw external stream"
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

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "[a b] | example collect-external",
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
        let stream = input.into_iter().map(|value| {
            value
                .as_str()
                .map(|str| str.as_bytes())
                .or_else(|_| value.as_binary())
                .map(|bin| bin.to_vec())
        });
        Ok(PipelineData::ExternalStream {
            stdout: Some(RawStream::new(Box::new(stream), None, call.head, None)),
            stderr: None,
            exit_code: None,
            span: call.head,
            metadata: None,
            trim_end_newline: false,
        })
    }
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;
    PluginTest::new("example", ExamplePlugin.into())?.test_command_examples(&CollectExternal)
}
