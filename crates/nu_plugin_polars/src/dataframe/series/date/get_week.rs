use crate::{
    values::{to_pipeline_data, CustomValueSupport},
    PolarsPlugin,
};

use super::super::super::values::NuDataFrame;

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type,
};
use polars::{
    prelude::{DatetimeMethods, IntoSeries, NamedFrom},
    series::Series,
};

#[derive(Clone)]
pub struct GetWeek;

impl PluginCommand for GetWeek {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars get-week"
    }

    fn usage(&self) -> &str {
        "Gets week from date."
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
            description: "Returns week from a date",
            example: r#"let dt = ('2020-08-04T16:39:18+00:00' | into datetime --timezone 'UTC');
    let df = ([$dt $dt] | polars into-df);
    $df | polars get-week"#,
            result: Some(
                NuDataFrame::try_from_series(Series::new("0", &[32i8, 32]), Span::test_data())
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

    let casted = series.datetime().map_err(|e| ShellError::GenericError {
        error: "Error casting to datetime type".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let res = casted.week().into_series();

    let df = NuDataFrame::try_from_series_vec(vec![res], call.head)?;
    to_pipeline_data(plugin, engine, call.head, df)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&GetWeek)
    }
}
