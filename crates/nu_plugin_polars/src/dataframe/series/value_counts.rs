use crate::{values::CustomValueSupport, PolarsPlugin};

use super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type, Value,
};

use polars::prelude::SeriesMethods;

#[derive(Clone)]
pub struct ValueCount;

impl PluginCommand for ValueCount {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars value-counts"
    }

    fn usage(&self) -> &str {
        "Returns a dataframe with the counts for unique values in series."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Calculates value counts",
            example: "[5 5 5 5 6 6] | polars into-df | polars value-counts",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "0".to_string(),
                            vec![Value::test_int(5), Value::test_int(6)],
                        ),
                        Column::new(
                            "count".to_string(),
                            vec![Value::test_int(4), Value::test_int(2)],
                        ),
                    ],
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
        command(plugin, engine, call, input).map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
    let series = df.as_series(call.head)?;

    let res = series
        .value_counts(false, false)
        .map_err(|e| ShellError::GenericError {
            error: "Error calculating value counts values".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: Some("The str-slice command can only be used with string columns".into()),
            inner: vec![],
        })?;

    let df: NuDataFrame = res.into();
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ValueCount)
    }
}
