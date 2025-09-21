use crate::PolarsPlugin;
use crate::values::{Column, CustomValueSupport, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct AllTrue;

impl PluginCommand for AllTrue {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars all-true"
    }

    fn description(&self) -> &str {
        "Returns true if all values are true."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Returns true if all values are true",
                example: "[true true true] | polars into-df | polars all-true",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "all_true".to_string(),
                            vec![Value::test_bool(true)],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Checks the result from a comparison",
                example: r#"let s = ([5 6 2 8] | polars into-df);
    let res = ($s > 9);
    $res | polars all-true"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "all_true".to_string(),
                            vec![Value::test_bool(false)],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
        ]
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

    let series = df.as_series(call.head)?;
    let bool = series.bool().map_err(|_| ShellError::GenericError {
        error: "Error converting to bool".into(),
        msg: "all-false only works with series of type bool".into(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let value = Value::bool(bool.all(), call.head);

    let df = NuDataFrame::try_from_columns(
        vec![Column::new("all_true".to_string(), vec![value])],
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
        test_polars_plugin_command(&AllTrue)
    }
}
