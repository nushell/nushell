use crate::dataframe::values::{utils::convert_columns_string, Column, NuDataFrame};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct DropNulls;

impl Command for DropNulls {
    fn name(&self) -> &str {
        "dfr drop-nulls"
    }

    fn usage(&self) -> &str {
        "Drops null values in dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
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
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "drop null values in dataframe",
                example: r#"let df = ([[a b]; [1 2] [3 0] [1 2]] | dfr into-df);
    let res = ($df.b / $df.b);
    let a = ($df | dfr with-column $res --name res);
    $a | dfr drop-nulls"#,
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
            Example {
                description: "drop null values in dataframe",
                example: r#"let s = ([1 2 0 0 3 4] | dfr into-df);
    ($s / $s) | dfr drop-nulls"#,
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
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        command(engine_state, stack, call, input)
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let columns: Option<Vec<Value>> = call.opt(engine_state, stack, 0)?;

    let (subset, col_span) = match columns {
        Some(cols) => {
            let (agg_string, col_span) = convert_columns_string(cols, call.head)?;
            (Some(agg_string), col_span)
        }
        None => (None, call.head),
    };

    let subset_slice = subset.as_ref().map(|cols| &cols[..]);

    df.as_ref()
        .drop_nulls(subset_slice)
        .map_err(|e| ShellError::GenericError {
            error: "Error dropping nulls".into(),
            msg: e.to_string(),
            span: Some(col_span),
            help: None,
            inner: vec![],
        })
        .map(|df| PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::super::WithColumn;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(DropNulls {}), Box::new(WithColumn {})])
    }
}
