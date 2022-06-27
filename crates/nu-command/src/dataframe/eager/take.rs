use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use polars::prelude::DataType;

use crate::dataframe::values::Column;

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct TakeDF;

impl Command for TakeDF {
    fn name(&self) -> &str {
        "take"
    }

    fn usage(&self) -> &str {
        "Creates new dataframe using the given indices"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "indices",
                SyntaxShape::Any,
                "list of indices used to take data",
            )
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Takes selected rows from dataframe",
                example: r#"let df = ([[a b]; [4 1] [5 2] [4 3]] | into df);
    let indices = ([0 2] | into df);
    $df | take $indices"#,
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_int(4), Value::test_int(4)],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_int(1), Value::test_int(3)],
                        ),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Takes selected rows from series",
                example: r#"let series = ([4 1 5 2 4 3] | into df);
    let indices = ([0 2] | into df);
    $series | take $indices"#,
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "0".to_string(),
                        vec![Value::test_int(4), Value::test_int(5)],
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
    let index_value: Value = call.req(engine_state, stack, 0)?;
    let index_span = index_value.span()?;
    let index = NuDataFrame::try_from_value(index_value)?.as_series(index_span)?;

    let casted = match index.dtype() {
        DataType::UInt32 | DataType::UInt64 | DataType::Int32 | DataType::Int64 => {
            index.cast(&DataType::UInt32).map_err(|e| {
                ShellError::GenericError(
                    "Error casting index list".into(),
                    e.to_string(),
                    Some(index_span),
                    None,
                    Vec::new(),
                )
            })
        }
        _ => Err(ShellError::GenericError(
            "Incorrect type".into(),
            "Series with incorrect type".into(),
            Some(call.head),
            Some("Consider using a Series with type int type".into()),
            Vec::new(),
        )),
    }?;

    let indices = casted.u32().map_err(|e| {
        ShellError::GenericError(
            "Error casting index list".into(),
            e.to_string(),
            Some(index_span),
            None,
            Vec::new(),
        )
    })?;

    NuDataFrame::try_from_pipeline(input, call.head).and_then(|df| {
        df.as_ref()
            .take(indices)
            .map_err(|e| {
                ShellError::GenericError(
                    "Error taking values".into(),
                    e.to_string(),
                    Some(call.head),
                    None,
                    Vec::new(),
                )
            })
            .map(|df| PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None))
    })
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(TakeDF {})])
    }
}
