use crate::{values::CustomValueSupport, PolarsPlugin};

use super::super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::prelude::IntoSeries;

#[derive(Clone)]
pub struct StrFTime;

impl PluginCommand for StrFTime {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars strftime"
    }

    fn usage(&self) -> &str {
        "Formats date based on string rule."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("fmt", SyntaxShape::String, "Format rule")
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Formats date",
            example: r#"let dt = ('2020-08-04T16:39:18+00:00' | into datetime --timezone 'UTC');
    let df = ([$dt $dt] | polars into-df);
    $df | polars strftime "%Y/%m/%d""#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "0".to_string(),
                        vec![
                            Value::test_string("2020/08/04"),
                            Value::test_string("2020/08/04"),
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
        command(plugin, engine, call, input).map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let fmt: String = call.req(0)?;

    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
    let series = df.as_series(call.head)?;

    let casted = series.datetime().map_err(|e| ShellError::GenericError {
        error: "Error casting to date".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: Some("The str-slice command can only be used with string columns".into()),
        inner: vec![],
    })?;

    let res = casted
        .strftime(&fmt)
        .map_err(|e| ShellError::GenericError {
            error: "Error formatting datetime".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?
        .into_series();

    let df = NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command_with_decls;
    use nu_command::IntoDatetime;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command_with_decls(&StrFTime, vec![Box::new(IntoDatetime)])
    }
}
