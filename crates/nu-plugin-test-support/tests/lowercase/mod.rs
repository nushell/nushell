use nu_plugin::*;
use nu_plugin_test_support::PluginTest;
use nu_protocol::{
    Example, IntoInterruptiblePipelineData, LabeledError, PipelineData, ShellError, Signals,
    Signature, Span, Type, Value,
};

struct LowercasePlugin;
struct Lowercase;

impl PluginCommand for Lowercase {
    type Plugin = LowercasePlugin;

    fn name(&self) -> &str {
        "lowercase"
    }

    fn description(&self) -> &str {
        "Convert each string in a stream to lowercase"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).input_output_type(
            Type::List(Type::String.into()),
            Type::List(Type::String.into()),
        )
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: r#"[Hello wORLD] | lowercase"#,
            description: "Lowercase a list of strings",
            result: Some(Value::test_list(vec![
                Value::test_string("hello"),
                Value::test_string("world"),
            ])),
        }]
    }

    fn run(
        &self,
        _plugin: &LowercasePlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let span = call.head;
        Ok(input.map(
            move |value| {
                value
                    .as_str()
                    .map(|string| Value::string(string.to_lowercase(), span))
                    // Errors in a stream should be returned as values.
                    .unwrap_or_else(|err| Value::error(err, span))
            },
            &Signals::empty(),
        )?)
    }
}

impl Plugin for LowercasePlugin {
    fn version(&self) -> String {
        "0.0.0".into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(Lowercase)]
    }
}

#[test]
fn test_lowercase_using_eval_with() -> Result<(), ShellError> {
    let result = PluginTest::new("lowercase", LowercasePlugin.into())?.eval_with(
        "lowercase",
        vec![Value::test_string("HeLlO wOrLd")]
            .into_pipeline_data(Span::test_data(), Signals::empty()),
    )?;

    assert_eq!(
        Value::test_list(vec![Value::test_string("hello world")]),
        result.into_value(Span::test_data())?
    );

    Ok(())
}

#[test]
fn test_lowercase_examples() -> Result<(), ShellError> {
    PluginTest::new("lowercase", LowercasePlugin.into())?.test_command_examples(&Lowercase)
}
