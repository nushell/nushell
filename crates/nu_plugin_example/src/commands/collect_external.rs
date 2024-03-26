use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, LabeledError, PipelineData, PluginExample, PluginSignature, RawStream, Type, Value,
};

use crate::Example;

/// `<list<string>> | example collect-external`
pub struct CollectExternal;

impl PluginCommand for CollectExternal {
    type Plugin = Example;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("example collect-external")
            .usage("Example transformer to raw external stream")
            .search_terms(vec!["example".into()])
            .input_output_types(vec![
                (Type::List(Type::String.into()), Type::String),
                (Type::List(Type::Binary.into()), Type::Binary),
            ])
            .plugin_examples(vec![PluginExample {
                example: "[a b] | example collect-external".into(),
                description: "collect strings into one stream".into(),
                result: Some(Value::test_string("ab")),
            }])
            .category(Category::Experimental)
    }

    fn run(
        &self,
        _plugin: &Example,
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
    PluginTest::new("example", Example.into())?.test_command_examples(&CollectExternal)
}
