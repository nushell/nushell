use crate::{PolarsPlugin, values::CustomValueSupport};

use crate::values::{Column, NuDataFrame, PolarsPluginType};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Value,
};

#[derive(Clone)]
pub struct NNull;

impl PluginCommand for NNull {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars count-null"
    }

    fn description(&self) -> &str {
        "Counts null values."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (
                    PolarsPluginType::NuDataFrame.into(),
                    PolarsPluginType::NuDataFrame.into(),
                ),
                (
                    PolarsPluginType::NuLazyFrame.into(),
                    PolarsPluginType::NuLazyFrame.into(),
                ),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Counts null values",
            example: r#"let s = ([1 1 0 0 3 3 4] | polars into-df);
    ($s / $s) | polars count-null"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "count_null".to_string(),
                        vec![Value::test_int(2)],
                    )],
                    None,
                )
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        command(plugin, engine, call, input)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;

    let res = df.as_series(call.head)?.null_count();
    let value = Value::int(res as i64, call.head);

    let df = NuDataFrame::try_from_columns(
        vec![Column::new("count_null".to_string(), vec![value])],
        None,
    )?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&NNull)
    }
}
