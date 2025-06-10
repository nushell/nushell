use crate::PolarsPlugin;
use crate::values::NuDataFrame;

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct ColumnsDF;

impl PluginCommand for ColumnsDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars columns"
    }

    fn description(&self) -> &str {
        "Show dataframe columns."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Custom("dataframe".into()), Type::Any)
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Dataframe columns",
            example: "[[a b]; [1 2] [3 4]] | polars into-df | polars columns",
            result: Some(Value::list(
                vec![Value::test_string("a"), Value::test_string("b")],
                Span::test_data(),
            )),
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        command(plugin, call, input)
            .map_err(|e| e.into())
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn command(
    plugin: &PolarsPlugin,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;

    let names: Vec<Value> = df
        .as_ref()
        .get_column_names()
        .iter()
        .map(|v| Value::string(v.as_str(), call.head))
        .collect();

    let names = Value::list(names, call.head);

    Ok(PipelineData::Value(names, None))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ColumnsDF)
    }
}
