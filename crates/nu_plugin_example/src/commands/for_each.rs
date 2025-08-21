use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, SyntaxShape, Type};

use crate::ExamplePlugin;

/// `<list> | example for-each { |value| ... }`
pub struct ForEach;

impl PluginCommand for ForEach {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example for-each"
    }

    fn description(&self) -> &str {
        "Example execution of a closure with a stream"
    }

    fn extra_description(&self) -> &str {
        "Prints each value the closure returns to stderr"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::list(Type::Any), Type::Nothing)
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The closure to run for each input value",
            )
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "ls | get name | example for-each { |f| ^file $f }",
            description: "example with an external command",
            result: None,
        }]
    }

    fn run(
        &self,
        _plugin: &ExamplePlugin,
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
        Ok(PipelineData::empty())
    }
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;
    PluginTest::new("example", ExamplePlugin.into())?.test_command_examples(&ForEach)
}
