use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

use crate::dataframe::values::{utils::convert_columns_string, Column};

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct SortDF;

impl Command for SortDF {
    fn name(&self) -> &str {
        "dfr sort"
    }

    fn usage(&self) -> &str {
        "Creates new sorted dataframe or series"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("reverse", "invert sort", Some('r'))
            .rest("rest", SyntaxShape::Any, "column names to sort dataframe")
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create new sorted dataframe",
                example: "[[a b]; [3 4] [1 2]] | dfr to-df | dfr sort a",
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_int(1), Value::test_int(3)],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_int(2), Value::test_int(4)],
                        ),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Create new sorted series",
                example: "[3 4 1 2] | dfr to-df | dfr sort",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "0".to_string(),
                        vec![
                            Value::test_int(1),
                            Value::test_int(2),
                            Value::test_int(3),
                            Value::test_int(4),
                        ],
                    )])
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
    let reverse = call.has_flag("reverse");

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    if df.is_series() {
        let columns = df.as_ref().get_column_names();

        df.as_ref()
            .sort(columns, reverse)
            .map_err(|e| {
                ShellError::SpannedLabeledError(
                    "Error sorting dataframe".into(),
                    e.to_string(),
                    call.head,
                )
            })
            .map(|df| PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None))
    } else {
        let columns: Vec<Value> = call.rest(engine_state, stack, 0)?;

        if !columns.is_empty() {
            let (col_string, col_span) = convert_columns_string(columns, call.head)?;

            df.as_ref()
                .sort(&col_string, reverse)
                .map_err(|e| {
                    ShellError::SpannedLabeledError(
                        "Error sorting dataframe".into(),
                        e.to_string(),
                        col_span,
                    )
                })
                .map(|df| {
                    PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None)
                })
        } else {
            Err(ShellError::SpannedLabeledError(
                "Missing columns".into(),
                "missing column name to perform sort".into(),
                call.head,
            ))
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(SortDF {})])
    }
}
