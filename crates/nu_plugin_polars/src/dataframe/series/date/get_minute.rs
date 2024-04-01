use crate::{
    values::{to_pipeline_data, CustomValueSupport},
    PolarsPlugin,
};

use super::super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type, Value,
};
use polars::prelude::{DatetimeMethods, IntoSeries};

#[derive(Clone)]
pub struct GetMinute;

impl PluginCommand for GetMinute {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars get-minute"
    }

    fn usage(&self) -> &str {
        "Gets minute from date."
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
            description: "Returns minute from a date",
            example: r#"let dt = ('2020-08-04T16:39:18+00:00' | into datetime --timezone 'UTC');
    let df = ([$dt $dt] | polars into-df);
    $df | polars get-minute"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "0".to_string(),
                        vec![Value::test_int(39), Value::test_int(39)],
                    )],
                    None,
                )
                .expect("simple df for test should not fail")
                .base_value(Span::test_data())
                .expect("rendering base value should not fail"),
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

    let res = casted.minute().into_series();

    let df = NuDataFrame::try_from_series_columns(vec![res], call.head)?;
    to_pipeline_data(plugin, engine, call.head, df)
}

// todo: fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::super::super::IntoDatetime;
//     use super::super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![Box::new(GetMinute {}), Box::new(IntoDatetime {})])
//     }
// }
