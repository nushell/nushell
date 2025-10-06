use crate::PolarsPlugin;
use crate::values::{CustomValueSupport, NuDataFrame, PolarsPluginType};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape,
};

use polars::df;
use polars::prelude::SeriesMethods;

#[derive(Clone)]
pub struct ValueCount;

impl PluginCommand for ValueCount {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars value-counts"
    }

    fn description(&self) -> &str {
        "Returns a dataframe with the counts for unique values in series."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "column",
                SyntaxShape::String,
                "Provide a custom name for the count column",
                Some('c'),
            )
            .switch("sort", "Whether or not values should be sorted", Some('s'))
            .switch(
                "parallel",
                "Use multiple threads when processing",
                Some('p'),
            )
            .named(
                "normalize",
                SyntaxShape::String,
                "Normalize the counts",
                Some('n'),
            )
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
            description: "Calculates value counts",
            example: "[5 5 5 5 6 6] | polars into-df | polars value-counts | polars sort-by count",
            result: Some(
                NuDataFrame::from(
                    df!(
                        "0" => &[6i64, 5],
                        "count" => &[2i64, 4],
                    )
                    .expect("should not fail"),
                )
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
    let column = call.get_flag("column")?.unwrap_or("count".to_string());
    let parallel = call.has_flag("parallel")?;
    let sort = call.has_flag("sort")?;
    let normalize = call.has_flag("normalize")?;
    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
    let series = df.as_series(call.head)?;

    let res = series
        .value_counts(sort, parallel, column.into(), normalize)
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
