use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, LabeledError, Signature, SyntaxShape, Value};

use crate::ExamplePlugin;

pub struct Three;

impl SimplePluginCommand for Three {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example three"
    }

    fn description(&self) -> &str {
        "Plugin test example 3. Returns labeled error"
    }

    fn signature(&self) -> Signature {
        // The signature defines the usage of the command inside Nu, and also automatically
        // generates its help page.
        Signature::build(self.name())
            .required("a", SyntaxShape::Int, "Required integer value.")
            .required("b", SyntaxShape::String, "Required string value.")
            .switch("flag", "A flag for the signature.", Some('f'))
            .optional("opt", SyntaxShape::Int, "Optional number.")
            .named("named", SyntaxShape::String, "Named string.", Some('n'))
            .rest("rest", SyntaxShape::String, "Rest value string.")
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
