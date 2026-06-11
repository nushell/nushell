use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, Type, Value};

use crate::PolarsPlugin;

#[derive(Clone)]
pub struct MathCmd;

impl PluginCommand for MathCmd {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars math"
    }

    fn description(&self) -> &str {
        "Collection of math functions to be applied on column expressions."
    }

    fn extra_description(&self) -> &str {
        r#"
You must use one of the subcommands below. Using this command as-is will only produce this help message.

See https://docs.pola.rs/api/python/stable/reference/expressions/computation.html for more information.
"#
    }

    fn signature(&self) -> Signature {
        Signature::build("polars math")
            .category(Category::Custom("dataframe".into()))
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        Ok(PipelineData::value(
            Value::string(engine.get_help()?, call.head),
            None,
        ))
    }
}
