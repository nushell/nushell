use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, PolarsPluginType},
};

use super::super::super::values::{Column, NuDataFrame};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Value,
};
use polars::prelude::IntoSeries;

use std::ops::Not;

#[derive(Clone)]
pub struct NotSeries;

impl PluginCommand for NotSeries {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars not"
    }

    fn description(&self) -> &str {
        "Inverts boolean mask."
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
            description: "Inverts boolean mask",
            example: "[true false true] | polars into-df | polars not",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "0".to_string(),
                        vec![
                            Value::test_bool(false),
                            Value::test_bool(true),
                            Value::test_bool(false),
                        ],
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
        let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
        command(plugin, engine, call, df)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let series = df.as_series(call.head)?;

    let bool = series.bool().map_err(|e| ShellError::GenericError {
        error: "Error inverting mask".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let res = bool.not();

    let df = NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&NotSeries)
    }
}
