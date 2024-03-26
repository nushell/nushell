use crate::dataframe::values::{utils::convert_columns, Column, NuDataFrame};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct DropDF;

impl Command for DropDF {
    fn name(&self) -> &str {
        "dfr drop"
    }

    fn usage(&self) -> &str {
        "Creates a new dataframe by dropping the selected columns."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest("rest", SyntaxShape::Any, "column names to be dropped")
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "drop column a",
            example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr drop a",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "b".to_string(),
                        vec![Value::test_int(2), Value::test_int(4)],
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
    let columns: Vec<Value> = call.rest(engine_state, stack, 0)?;
    let (col_string, col_span) = convert_columns(columns, call.head)?;

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let new_df = col_string
        .first()
        .ok_or_else(|| ShellError::GenericError {
            error: "Empty names list".into(),
            msg: "No column names were found".into(),
            span: Some(col_span),
            help: None,
            inner: vec![],
        })
        .and_then(|col| {
            df.as_ref()
                .drop(&col.item)
                .map_err(|e| ShellError::GenericError {
                    error: "Error dropping column".into(),
                    msg: e.to_string(),
                    span: Some(col.span),
                    help: None,
                    inner: vec![],
                })
        })?;

    // If there are more columns in the drop selection list, these
    // are added from the resulting dataframe
    col_string
        .iter()
        .skip(1)
        .try_fold(new_df, |new_df, col| {
            new_df
                .drop(&col.item)
                .map_err(|e| ShellError::GenericError {
                    error: "Error dropping column".into(),
                    msg: e.to_string(),
                    span: Some(col.span),
                    help: None,
                    inner: vec![],
                })
        })
        .map(|df| PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(DropDF {})])
    }
}
