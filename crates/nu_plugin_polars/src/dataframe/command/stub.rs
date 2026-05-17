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

    fn description(&self) -> &str {
        "Operate with data in a dataframe format."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("polars")
            .category(Category::Custom("dataframe".into()))
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn extra_description(&self) -> &str {
        r#"
You must use one of the subcommands below. Using this command as-is will only produce this help message.

The following are the main datatypes (wrapped from Polars) that are used by these subcommands:

Lazy and Strict dataframes (called `NuLazyFrame` and `NuDataFrame` in error messages) are the main
data structure.

Expressions, representing various column operations (called `NuExpression`), are passed to many commands such as
`polars filter` or `polars with-column`. Most nushell operators are supported in these expressions, importantly
arithmetic, comparison and boolean logical.

Groupbys (`NuLazyGroupBy`), the output of a `polars group-by`, represent a grouped dataframe and are typically piped
to the `polars agg` command with some column expressions for aggregation which then returns a dataframe.
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
