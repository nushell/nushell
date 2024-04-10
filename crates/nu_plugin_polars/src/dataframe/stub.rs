use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, Type, Value};

use crate::PolarsPlugin;

#[derive(Clone)]
pub struct PolarsCmd;

impl PluginCommand for PolarsCmd {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars"
    }

    fn usage(&self) -> &str {
        "Operate with data in a dataframe format."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("polars")
            .category(Category::Custom("dataframe".into()))
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn extra_usage(&self) -> &str {
        "You must use one of the following subcommands. Using this command as-is will only produce this help message."
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        Ok(PipelineData::Value(
            Value::string(engine.get_help()?, call.head),
            None,
        ))
    }
}
