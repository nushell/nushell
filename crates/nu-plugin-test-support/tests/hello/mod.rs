//! Extended from `nu-plugin` examples.

use nu_plugin::*;
use nu_plugin_test_support::PluginTest;
use nu_protocol::{Example, LabeledError, ShellError, Signature, Type, Value};

struct HelloPlugin;
struct Hello;

impl Plugin for HelloPlugin {
    fn version(&self) -> String {
        "0.0.0".into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(Hello)]
    }
}

impl SimplePluginCommand for Hello {
    type Plugin = HelloPlugin;

    fn name(&self) -> &str {
        "hello"
    }

    fn description(&self) -> &str {
        "Print a friendly greeting"
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self)).input_output_type(Type::Nothing, Type::String)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: "hello",
            description: "Print a friendly greeting",
            result: Some(Value::test_string("Hello, World!")),
        }]
    }

    fn run(
        &self,
        _plugin: &HelloPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        Ok(Value::string("Hello, World!".to_owned(), call.head))
    }
}

#[test]
fn test_specified_examples() -> Result<(), ShellError> {
    PluginTest::new("hello", HelloPlugin.into())?.test_command_examples(&Hello)
}

#[test]
fn test_an_error_causing_example() -> Result<(), ShellError> {
    let result = PluginTest::new("hello", HelloPlugin.into())?.test_examples(&[Example {
        example: "hello --unknown-flag",
        description: "Run hello with an unknown flag",
        result: Some(Value::test_string("Hello, World!")),
    }]);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_an_example_with_the_wrong_result() -> Result<(), ShellError> {
    let result = PluginTest::new("hello", HelloPlugin.into())?.test_examples(&[Example {
        example: "hello",
        description: "Run hello but the example result is wrong",
        result: Some(Value::test_string("Goodbye, World!")),
    }]);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_requiring_nu_cmd_lang_commands() -> Result<(), ShellError> {
    use nu_protocol::Span;

    let result = PluginTest::new("hello", HelloPlugin.into())?
        .eval("do { let greeting = hello; $greeting }")?
        .into_value(Span::test_data())?;

    assert_eq!(Value::test_string("Hello, World!"), result);

    Ok(())
}
