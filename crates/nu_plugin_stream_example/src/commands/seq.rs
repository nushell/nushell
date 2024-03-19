use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, PluginCommand};
use nu_protocol::{
    Category, ListStream, PipelineData, PluginExample, PluginSignature, SyntaxShape, Type, Value,
};

use crate::StreamExample;

/// `stream_example seq <first> <last>`
pub struct Seq;

impl PluginCommand for Seq {
    type Plugin = StreamExample;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("stream_example seq")
            .usage("Example stream generator for a list of values")
            .search_terms(vec!["example".into()])
            .required("first", SyntaxShape::Int, "first number to generate")
            .required("last", SyntaxShape::Int, "last number to generate")
            .input_output_type(Type::Nothing, Type::List(Type::Int.into()))
            .plugin_examples(vec![PluginExample {
                example: "stream_example seq 1 3".into(),
                description: "generate a sequence from 1 to 3".into(),
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                ])),
            }])
            .category(Category::Experimental)
    }

    fn run(
        &self,
        _plugin: &StreamExample,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let first: i64 = call.req(0)?;
        let last: i64 = call.req(1)?;
        let span = call.head;
        let iter = (first..=last).map(move |number| Value::int(number, span));
        let list_stream = ListStream::from_stream(iter, None);
        Ok(PipelineData::ListStream(list_stream, None))
    }
}
