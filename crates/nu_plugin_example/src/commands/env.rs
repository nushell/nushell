use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, SimplePluginCommand};
use nu_protocol::{Category, PluginSignature, SyntaxShape, Type, Value};

use crate::Example;

pub struct Env;

impl SimplePluginCommand for Env {
    type Plugin = Example;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("example env")
            .usage("Get environment variable(s)")
            .extra_usage("Returns all environment variables if no name provided")
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
            .search_terms(vec!["example".into(), "env".into()])
            .input_output_type(Type::Nothing, Type::Any)
    }

    fn run(
        &self,
        _plugin: &Example,
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
                Some(value) => Err(LabeledError {
                    label: "Invalid arguments".into(),
                    msg: "--cwd can't be used with --set".into(),
                    span: Some(value.span()),
                }),
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
