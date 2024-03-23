use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, LabeledError, PipelineData, PluginExample, PluginSignature, ShellError, Span,
    SyntaxShape, Type, Value,
};

use crate::PolarsPlugin;

use super::super::values::{Axis, Column, NuDataFrame};

#[derive(Clone)]
pub struct AppendDF;

impl PluginCommand for AppendDF {
    type Plugin = PolarsPlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("polars append")
            .usage("Appends a new dataframe.")
            .required("other", SyntaxShape::Any, "other dataframe to append")
            .switch("col", "append as new columns instead of rows", None)
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
            .plugin_examples(examples())
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        command(engine, call, input).map_err(LabeledError::from)
    }
}
fn examples() -> Vec<PluginExample> {
    vec![
        PluginExample {
            description: "Appends a dataframe as new columns".into(),
            example: r#"let a = ([[a b]; [1 2] [3 4]] | polars into-df);
    $a | polars append $a"#
                .into(),
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
        PluginExample {
            description: "Appends a dataframe merging at the end of columns".into(),
            example:
                r#"let a = ([[a b]; [1 2] [3 4]] | polars into-df); $a | polars append $a --col"#
                    .into(),
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

fn command(
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
    let df_other = NuDataFrame::try_from_value(other)?;
    let df = NuDataFrame::try_from_pipeline(input, call.head)?;
    let df = df.append_df(&df_other, axis, call.head)?;

    Ok(PipelineData::Value(
        df.insert_cache(engine)?.into_value(call.head),
        None,
    ))
}

// todo - fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![Box::new(AppendDF {})])
//     }
// }
