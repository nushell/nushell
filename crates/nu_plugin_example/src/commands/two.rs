use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, IntoValue, LabeledError, Signature, SyntaxShape, Value};

use crate::ExamplePlugin;

pub struct Two;

impl SimplePluginCommand for Two {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example two"
    }

    fn usage(&self) -> &str {
        "Plugin test example 2. Returns list of records"
    }

    fn signature(&self) -> Signature {
        // The signature defines the usage of the command inside Nu, and also automatically
        // generates its help page.
        Signature::build(self.name())
            .required("a", SyntaxShape::Int, "required integer value")
            .required("b", SyntaxShape::String, "required string value")
            .switch("flag", "a flag for the signature", Some('f'))
            .optional("opt", SyntaxShape::Int, "Optional number")
            .named("named", SyntaxShape::String, "named string", Some('n'))
            .rest("rest", SyntaxShape::String, "rest value string")
            .category(Category::Experimental)
    }

    fn run(
        &self,
        plugin: &ExamplePlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        plugin.print_values(2, call, input)?;

        // Use the IntoValue derive macro and trait to easily design output data.
        #[derive(IntoValue)]
        struct Output {
            one: i64,
            two: i64,
            three: i64,
        }

        let vals = (0..10i64)
            .map(|i| {
                Output {
                    one: i,
                    two: 2 * i,
                    three: 3 * i,
                }
                .into_value(call.head)
            })
            .collect();

        Ok(Value::list(vals, call.head))
    }
}
