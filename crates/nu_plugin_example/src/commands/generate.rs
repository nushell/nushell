use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, LabeledError, PipelineData, Signals,
    Signature, SyntaxShape, Type, Value,
};

use crate::ExamplePlugin;

/// `example generate <initial> { |previous| {out: ..., next: ...} }`
pub struct Generate;

impl PluginCommand for Generate {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example generate"
    }

    fn description(&self) -> &str {
        "Example execution of a closure to produce a stream"
    }

    fn extra_description(&self) -> &str {
        "See the builtin `generate` command"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Nothing, Type::list(Type::Any))
            .required(
                "initial",
                SyntaxShape::Any,
                "The initial value to pass to the closure",
            )
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The closure to run to generate values",
            )
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "example generate 0 { |i| if $i <= 10 { {out: $i, next: ($i + 2)} } }",
            description: "Generate a sequence of numbers",
            result: Some(Value::test_list(
                [0, 2, 4, 6, 8, 10]
                    .into_iter()
                    .map(Value::test_int)
                    .collect(),
            )),
        }]
    }

    fn run(
        &self,
        _plugin: &ExamplePlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let head = call.head;
        let engine = engine.clone();
        let initial: Value = call.req(0)?;
        let closure = call.req(1)?;

        let mut next = (!initial.is_nothing()).then_some(initial);

        Ok(std::iter::from_fn(move || {
            next.take()
                .and_then(|value| {
                    engine
                        .eval_closure(&closure, vec![value.clone()], Some(value))
                        .and_then(|record| {
                            if record.is_nothing() {
                                Ok(None)
                            } else {
                                let record = record.as_record()?;
                                next = record.get("next").cloned();
                                Ok(record.get("out").cloned())
                            }
                        })
                        .transpose()
                })
                .map(|result| result.unwrap_or_else(|err| Value::error(err, head)))
        })
        .into_pipeline_data(head, Signals::empty()))
    }
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_cmd_lang::If;
    use nu_plugin_test_support::PluginTest;
    PluginTest::new("example", ExamplePlugin.into())?
        .add_decl(Box::new(If))?
        .test_command_examples(&Generate)
}
