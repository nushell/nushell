use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, LabeledError, Signature, SyntaxShape, Value};

use crate::ExamplePlugin;

pub struct Three;

impl SimplePluginCommand for Three {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example three"
    }

    fn usage(&self) -> &str {
        "Plugin test example 3. Returns labeled error"
    }

    fn signature(&self) -> Signature {
        // The signature defines the usage of the command inside Nu, and also automatically
        // generates its help page.
        Signature::build(self.name())
            .required_positional_arg("a", SyntaxShape::Int, "required integer value")
            .required_positional_arg("b", SyntaxShape::String, "required string value")
            .optional_named_flag_arg("flag", "a flag for the signature", Some('f'))
            .optional_position_arg("opt", SyntaxShape::Int, "Optional number")
            .named_flag_arg("named", SyntaxShape::String, "named string", Some('n'))
            .rest_positional_arg("rest", SyntaxShape::String, "rest value string")
            .category(Category::Experimental)
    }

    fn run(
        &self,
        plugin: &ExamplePlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        plugin.print_values(3, call, input)?;

        Err(LabeledError::new("ERROR from plugin")
            .with_label("error message pointing to call head span", call.head))
    }
}
