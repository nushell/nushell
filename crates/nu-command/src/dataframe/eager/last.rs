use super::super::values::{utils::DEFAULT_ROWS, Column, NuDataFrame};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct LastDF;

impl Command for LastDF {
    fn name(&self) -> &str {
        "last"
    }

    fn usage(&self) -> &str {
        "Creates new dataframe with tail rows or creates a last expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional("rows", SyntaxShape::Int, "Number of rows for tail")
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create new dataframe with last rows",
            example: "[[a b]; [1 2] [3 4]] | into df | last 1",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new("a".to_string(), vec![Value::test_int(3)]),
                    Column::new("b".to_string(), vec![Value::test_int(4)]),
                ])
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
        let df = NuDataFrame::try_from_pipeline(input, call.head)?;
        command(engine_state, stack, call, df)
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let rows: Option<usize> = call.opt(engine_state, stack, 0)?;
    let rows = rows.unwrap_or(DEFAULT_ROWS);

    let res = df.as_ref().tail(Some(rows));
    Ok(PipelineData::Value(
        NuDataFrame::dataframe_into_value(res, call.head),
        None,
    ))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(LastDF {})])
    }
}
