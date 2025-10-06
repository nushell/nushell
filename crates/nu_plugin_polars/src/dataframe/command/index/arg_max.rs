use crate::{PolarsPlugin, values::CustomValueSupport};

use crate::values::{Column, NuDataFrame, PolarsPluginType};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Value,
};
use polars::prelude::{ArgAgg, IntoSeries, NewChunkedArray, UInt32Chunked};

#[derive(Clone)]
pub struct ArgMax;

impl PluginCommand for ArgMax {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars arg-max"
    }

    fn description(&self) -> &str {
        "Return index for max value in series."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["argmax", "maximum", "most", "largest", "greatest"]
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
            description: "Returns index for max value",
            example: "[1 3 2] | polars into-df | polars arg-max",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new("arg_max".to_string(), vec![Value::test_int(1)])],
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

    let res = series.arg_max();
    let chunked = match res {
        Some(index) => UInt32Chunked::from_slice("arg_max".into(), &[index as u32]),
        None => UInt32Chunked::from_slice("arg_max".into(), &[]),
    };

    let res = chunked.into_series();
    let df = NuDataFrame::try_from_series_vec(vec![res], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ArgMax)
    }
}
