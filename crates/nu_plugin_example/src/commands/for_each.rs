use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, PluginCommand};
use nu_protocol::{Category, PipelineData, PluginExample, PluginSignature, SyntaxShape, Type};

use crate::Example;

/// `<list> | example for-each { |value| ... }`
pub struct ForEach;

impl PluginCommand for ForEach {
    type Plugin = Example;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("example for-each")
            .usage("Example execution of a closure with a stream")
            .extra_usage("Prints each value the closure returns to stderr")
            .input_output_type(Type::ListStream, Type::Nothing)
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The closure to run for each input value",
            )
            .plugin_examples(vec![PluginExample {
                example: "ls | get name | example for-each { |f| ^file $f }".into(),
                description: "example with an external command".into(),
                result: None,
            }])
            .category(Category::Experimental)
    }

    fn run(
        &self,
        _plugin: &Example,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let closure = call.req(0)?;
        let config = engine.get_config()?;
        for value in input {
            let result = engine.eval_closure(&closure, vec![value.clone()], Some(value))?;
            eprintln!("{}", result.to_expanded_string(", ", &config));
        }
        Ok(PipelineData::Empty)
    }
}
