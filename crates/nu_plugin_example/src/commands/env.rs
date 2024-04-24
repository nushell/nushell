use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, LabeledError, Signature, SyntaxShape, Type, Value};

use crate::ExamplePlugin;

pub struct Env;

impl SimplePluginCommand for Env {
    type Plugin = ExamplePlugin;

    fn name(&self) -> &str {
        "example env"
    }

    fn usage(&self) -> &str {
        "Get environment variable(s)"
    }

    fn extra_usage(&self) -> &str {
        "Returns all environment variables if no name provided"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Experimental)
            .optional(
                "name",
                SyntaxShape::String,
                "The name of the environment variable to get",
            )
            .switch("cwd", "Get current working directory instead", None)
            .named(
                "set",
                SyntaxShape::Any,
                "Set an environment variable to the value",
                None,
            )
            .input_output_type(Type::Nothing, Type::Any)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["example", "env"]
    }

    fn run(
        &self,
        _plugin: &ExamplePlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        if call.has_flag("cwd")? {
            match call.get_flag_value("set") {
                None => {
                    // Get working directory
                    Ok(Value::string(engine.get_current_dir()?, call.head))
                }
                Some(value) => Err(LabeledError::new("Invalid arguments")
                    .with_label("--cwd can't be used with --set", value.span())),
            }
        } else if let Some(value) = call.get_flag_value("set") {
            // Set single env var
            let name = call.req::<String>(0)?;
            engine.add_env_var(name, value)?;
            Ok(Value::nothing(call.head))
        } else if let Some(name) = call.opt::<String>(0)? {
            // Get single env var
            Ok(engine
                .get_env_var(name)?
                .unwrap_or(Value::nothing(call.head)))
        } else {
            // Get all env vars, converting the map to a record
            Ok(Value::record(
                engine.get_env_vars()?.into_iter().collect(),
                call.head,
            ))
        }
    }
}
