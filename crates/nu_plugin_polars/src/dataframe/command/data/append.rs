use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

use crate::{
    PolarsPlugin,
    values::{Axis, Column, CustomValueSupport, NuDataFrame},
};

#[derive(Clone)]
pub struct AppendDF;

impl PluginCommand for AppendDF {
    type Plugin = PolarsPlugin;

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("other", SyntaxShape::Any, "other dataframe to append")
            .switch("col", "append as new columns instead of rows", Some('c'))
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
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

    fn name(&self) -> &str {
        "polars append"
    }

    fn description(&self) -> &str {
        "Appends a new dataframe."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Appends a dataframe as new columns",
                example: r#"let a = ([[a b]; [1 2] [3 4]] | polars into-df);
    $a | polars append $a"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(1), Value::test_int(3)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(2), Value::test_int(4)],
                            ),
                            Column::new(
                                "a_x".to_string(),
                                vec![Value::test_int(1), Value::test_int(3)],
                            ),
                            Column::new(
                                "b_x".to_string(),
                                vec![Value::test_int(2), Value::test_int(4)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Appends a dataframe merging at the end of columns",
                example: r#"let a = ([[a b]; [1 2] [3 4]] | polars into-df); $a | polars append $a --col"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![
                                    Value::test_int(1),
                                    Value::test_int(3),
                                    Value::test_int(1),
                                    Value::test_int(3),
                                ],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![
                                    Value::test_int(2),
                                    Value::test_int(4),
                                    Value::test_int(2),
                                    Value::test_int(4),
                                ],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let other: Value = call.req(0)?;

    let axis = if call.has_flag("col")? {
        Axis::Column
    } else {
        Axis::Row
    };

    let df_other = NuDataFrame::try_from_value_coerce(plugin, &other, call.head)?;
    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
    let df = df.append_df(&df_other, axis, call.head)?;

    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&AppendDF)
    }
}
