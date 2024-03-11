use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, SimplePluginCommand};
use nu_protocol::{Category, PluginSignature, SyntaxShape, Type, Value};

use crate::Example;

pub struct NuExampleEnv;

impl SimplePluginCommand for NuExampleEnv {
    type Plugin = Example;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("nu-example-env")
            .usage("Get environment variable(s)")
            .extra_usage("Returns all environment variables if no name provided")
            .category(Category::Experimental)
            .optional(
                "name",
                SyntaxShape::String,
                "The name of the environment variable to get",
            )
            .switch("cwd", "Get current working directory instead", None)
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
            // Get working directory
            Ok(Value::string(engine.get_current_dir()?, call.head))
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
