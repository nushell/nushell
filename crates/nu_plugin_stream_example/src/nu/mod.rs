use crate::Example;
use nu_plugin::{EvaluatedCall, LabeledError, StreamingPlugin};
use nu_protocol::{
    Category, PipelineData, PluginExample, PluginSignature, Span, SyntaxShape, Type, Value,
};

impl StreamingPlugin for Example {
    fn signature(&self) -> Vec<PluginSignature> {
        let span = Span::unknown();
        vec![
            PluginSignature::build("stream_example")
                .usage("Examples for streaming plugins")
                .search_terms(vec!["example".into()])
                .category(Category::Experimental),
            PluginSignature::build("stream_example seq")
                .usage("Example stream generator for a list of values")
                .search_terms(vec!["example".into()])
                .required("first", SyntaxShape::Int, "first number to generate")
                .required("last", SyntaxShape::Int, "last number to generate")
                .input_output_type(Type::Nothing, Type::List(Type::Int.into()))
                .plugin_examples(vec![PluginExample {
                    example: "stream_example seq 1 3".into(),
                    description: "generate a sequence from 1 to 3".into(),
                    result: Some(Value::list(
                        vec![
                            Value::int(1, span),
                            Value::int(2, span),
                            Value::int(3, span),
                        ],
                        span,
                    )),
                }])
                .category(Category::Experimental),
            PluginSignature::build("stream_example sum")
                .usage("Example stream consumer for a list of values")
                .search_terms(vec!["example".into()])
                .input_output_types(vec![
                    (Type::List(Type::Int.into()), Type::Int),
                    (Type::List(Type::Float.into()), Type::Float),
                ])
                .plugin_examples(vec![PluginExample {
                    example: "seq 1 5 | stream_example sum".into(),
                    description: "sum values from 1 to 5".into(),
                    result: Some(Value::int(15, span)),
                }])
                .category(Category::Experimental),
            PluginSignature::build("stream_example collect-external")
                .usage("Example transformer to raw external stream")
                .search_terms(vec!["example".into()])
                .input_output_types(vec![
                    (Type::List(Type::String.into()), Type::String),
                    (Type::List(Type::Binary.into()), Type::Binary),
                ])
                .plugin_examples(vec![PluginExample {
                    example: "[a b] | stream_example collect-external".into(),
                    description: "collect strings into one stream".into(),
                    result: Some(Value::string("ab", span)),
                }])
                .category(Category::Experimental),
        ]
    }

    fn run(
        &mut self,
        name: &str,
        _config: &Option<Value>,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        match name {
            "stream_example" => Err(LabeledError {
                label: "No subcommand provided".into(),
                msg: "add --help here to see usage".into(),
                span: Some(call.head)
            }),
            "stream_example seq" => self.seq(call, input),
            "stream_example sum" => self.sum(call, input),
            "stream_example collect-external" => self.collect_external(call, input),
            _ => Err(LabeledError {
                label: "Plugin call with wrong name signature".into(),
                msg: "the signature used to call the plugin does not match any name in the plugin signature vector".into(),
                span: Some(call.head),
            }),
        }
    }
}
