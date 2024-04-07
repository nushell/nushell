use crate::{
    values::{to_pipeline_data, CustomValueSupport},
    PolarsPlugin,
};

use super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct AllFalse;

impl PluginCommand for AllFalse {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars all-false"
    }

    fn usage(&self) -> &str {
        "Returns true if all values are false."
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
        vec![
            Example {
                description: "Returns true if all values are false",
                example: "[false false false] | polars into-df | polars all-false",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "all_false".to_string(),
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
                example: r#"let s = ([5 6 2 10] | polars into-df);
    let res = ($s > 9);
    $res | polars all-false"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "all_false".to_string(),
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
    let bool = series.bool().map_err(|_| ShellError::GenericError {
        error: "Error converting to bool".into(),
        msg: "all-false only works with series of type bool".into(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let value = Value::bool(!bool.any(), call.head);

    let df = NuDataFrame::try_from_columns(
        vec![Column::new("all_false".to_string(), vec![value])],
        None,
    )?;
    to_pipeline_data(plugin, engine, call.head, df)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&AllFalse)
    }
}
