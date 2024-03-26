use crate::dataframe::values::{Column, NuDataFrame};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct ShapeDF;

impl Command for ShapeDF {
    fn name(&self) -> &str {
        "dfr shape"
    }

    fn usage(&self) -> &str {
        "Shows column and row size for a dataframe."
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
        vec![Example {
            description: "Shows row and column shape",
            example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr shape",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new("rows".to_string(), vec![Value::test_int(2)]),
                        Column::new("columns".to_string(), vec![Value::test_int(2)]),
                    ],
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
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let rows = Value::int(df.as_ref().height() as i64, call.head);

    let cols = Value::int(df.as_ref().width() as i64, call.head);

    let rows_col = Column::new("rows".to_string(), vec![rows]);
    let cols_col = Column::new("columns".to_string(), vec![cols]);

    NuDataFrame::try_from_columns(vec![rows_col, cols_col], None)
        .map(|df| PipelineData::Value(df.into_value(call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ShapeDF {})])
    }
}
