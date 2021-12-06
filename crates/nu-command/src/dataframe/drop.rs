use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

use super::values::utils::convert_columns;
use super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct DropDF;

impl Command for DropDF {
    fn name(&self) -> &str {
        "drop-df"
    }

    fn usage(&self) -> &str {
        "Creates a new dataframe by dropping the selected columns"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest("rest", SyntaxShape::Any, "column names to be dropped")
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "drop column a",
            example: "[[a b]; [1 2] [3 4]] | to df | drop-df a",
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "b".to_string(),
                    vec![2.into(), 4.into()],
                )])
                .expect("simple df for test should not fail")
                .into_value(Span::unknown()),
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
        .get(0)
        .ok_or_else(|| {
            ShellError::SpannedLabeledError(
                "Empty names list".into(),
                "No column names where found".into(),
                col_span,
            )
        })
        .and_then(|col| {
            df.as_ref().drop(&col.item).map_err(|e| {
                ShellError::SpannedLabeledError(
                    "Error dropping column".into(),
                    e.to_string(),
                    col.span,
                )
            })
        })?;

    // If there are more columns in the drop selection list, these
    // are added from the resulting dataframe
    col_string
        .iter()
        .skip(1)
        .try_fold(new_df, |new_df, col| {
            new_df.drop(&col.item).map_err(|e| {
                ShellError::SpannedLabeledError(
                    "Error dropping column".into(),
                    e.to_string(),
                    col.span,
                )
            })
        })
        .map(|df| PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(DropDF {})
    }
}
