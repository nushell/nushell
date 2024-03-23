use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, LabeledError, PipelineData, PluginExample, PluginSignature, ShellError, Span,
    SyntaxShape, Type, Value,
};

use crate::PolarsDataFramePlugin;

use super::super::values::utils::convert_columns_string;
use super::super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct DropNulls;

impl PluginCommand for DropNulls {
    type Plugin = PolarsDataFramePlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("polars drop-nulls")
            .usage("Drops null values in dataframe.")
            .optional(
                "subset",
                SyntaxShape::Table(vec![]),
                "subset of columns to drop nulls",
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
            .plugin_examples(vec![
                PluginExample {
                    description: "drop null values in dataframe".into(),
                    example: r#"let df = ([[a b]; [1 2] [3 0] [1 2]] | polars into-df);
    let res = ($df.b / $df.b);
    let a = ($df | polars with-column $res --name res);
    $a | polars drop-nulls"#
                        .into(),
                    result: Some(
                        NuDataFrame::try_from_columns(
                            vec![
                                Column::new(
                                    "a".to_string(),
                                    vec![Value::test_int(1), Value::test_int(1)],
                                ),
                                Column::new(
                                    "b".to_string(),
                                    vec![Value::test_int(2), Value::test_int(2)],
                                ),
                                Column::new(
                                    "res".to_string(),
                                    vec![Value::test_int(1), Value::test_int(1)],
                                ),
                            ],
                            None,
                        )
                        .expect("simple df for test should not fail")
                        .into_value(Span::test_data()),
                    ),
                },
                PluginExample {
                    description: "drop null values in dataframe".into(),
                    example: r#"let s = ([1 2 0 0 3 4] | polars into-df);
    ($s / $s) | polars drop-nulls"#
                        .into(),
                    result: Some(
                        NuDataFrame::try_from_columns(
                            vec![Column::new(
                                "div_0_0".to_string(),
                                vec![
                                    Value::test_int(1),
                                    Value::test_int(1),
                                    Value::test_int(1),
                                    Value::test_int(1),
                                ],
                            )],
                            None,
                        )
                        .expect("simple df for test should not fail")
                        .into_value(Span::test_data()),
                    ),
                },
            ])
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

fn command(
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let columns: Option<Vec<Value>> = call.opt(0)?;

    let (subset, col_span) = match columns {
        Some(cols) => {
            let (agg_string, col_span) = convert_columns_string(cols, call.head)?;
            (Some(agg_string), col_span)
        }
        None => (None, call.head),
    };

    let subset_slice = subset.as_ref().map(|cols| &cols[..]);

    let polars_df = df
        .as_ref()
        .drop_nulls(subset_slice)
        .map_err(|e| ShellError::GenericError {
            error: "Error dropping nulls".into(),
            msg: e.to_string(),
            span: Some(col_span),
            help: None,
            inner: vec![],
        })?;
    let df = NuDataFrame::new(false, polars_df);
    Ok(PipelineData::Value(
        df.insert_cache(engine)?.into_value(call.head),
        None,
    ))
}

// todo - fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::super::WithColumn;
//     use super::*;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![Box::new(DropNulls {}), Box::new(WithColumn {})])
//     }
// }
