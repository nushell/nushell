//! Extended from `nu-plugin` examples.

use nu_plugin::*;
use nu_plugin_test_support::PluginTest;
use nu_protocol::{LabeledError, PluginExample, PluginSignature, ShellError, Type, Value};

struct HelloPlugin;
struct Hello;

impl Plugin for HelloPlugin {
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(Hello)]
    }
}

impl SimplePluginCommand for Hello {
    type Plugin = HelloPlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("hello")
            .input_output_type(Type::Nothing, Type::String)
            .plugin_examples(vec![PluginExample {
                example: "hello".into(),
                description: "Print a friendly greeting".into(),
                result: Some(Value::test_string("Hello, World!")),
            }])
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
    let result = PluginTest::new("hello", HelloPlugin.into())?.test_examples(&[PluginExample {
        example: "hello --unknown-flag".into(),
        description: "Run hello with an unknown flag".into(),
        result: Some(Value::test_string("Hello, World!")),
    }]);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_an_example_with_the_wrong_result() -> Result<(), ShellError> {
    let result = PluginTest::new("hello", HelloPlugin.into())?.test_examples(&[PluginExample {
        example: "hello".into(),
        description: "Run hello but the example result is wrong".into(),
        result: Some(Value::test_string("Goodbye, World!")),
    }]);
    assert!(result.is_err());
    Ok(())
}
