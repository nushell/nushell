use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, SimplePluginCommand};
use nu_protocol::{record, Category, PluginSignature, SyntaxShape, Value};

use crate::Example;

pub struct NuExample2;

impl SimplePluginCommand for NuExample2 {
    type Plugin = Example;

    fn signature(&self) -> PluginSignature {
        // The signature defines the usage of the command inside Nu, and also automatically
        // generates its help page.
        PluginSignature::build("nu-example-2")
            .usage("PluginSignature test 2 for plugin. Returns list of records")
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
        plugin: &Example,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        plugin.print_values(2, call, input)?;

        let vals = (0..10i64)
            .map(|i| {
                let record = record! {
                    "one" => Value::int(i, call.head),
                    "two" => Value::int(2 * i, call.head),
                    "three" => Value::int(3 * i, call.head),
                };
                Value::record(record, call.head)
            })
            .collect();

        Ok(Value::list(vals, call.head))
    }
}
