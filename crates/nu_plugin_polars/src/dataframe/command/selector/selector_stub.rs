use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, LabeledError, PipelineData, Signature, Type, Value};

use crate::PolarsPlugin;

#[derive(Clone)]
pub struct SelectorCmd;

impl PluginCommand for SelectorCmd {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector"
    }

    fn description(&self) -> &str {
        "Create column selectors for use in polars commands."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("polars selector")
            .category(Category::Custom("expression".into()))
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn extra_description(&self) -> &str {
        r#"
You must use one of the subcommands below. Using this command as-is will only produce this help message.

Selectors are expressions that can be used to select columns in dataframes based on various criteria.
These selectors can be used with commands that accept column expressions, such as `polars select`,
`polars with-column`, and others.
"#
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
