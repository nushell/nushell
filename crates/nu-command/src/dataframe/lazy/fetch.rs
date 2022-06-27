use super::super::values::NuLazyFrame;
use crate::dataframe::values::{Column, NuDataFrame};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct LazyFetch;

impl Command for LazyFetch {
    fn name(&self) -> &str {
        "fetch"
    }

    fn usage(&self) -> &str {
        "collects the lazyframe to the selected rows"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "rows",
                SyntaxShape::Int,
                "number of rows to be fetched from lazyframe",
            )
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Fetch a rows from the dataframe",
            example: "[[a b]; [6 2] [4 2] [2 2]] | into df | fetch 2",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "a".to_string(),
                        vec![Value::test_int(6), Value::test_int(4)],
                    ),
                    Column::new(
                        "b".to_string(),
                        vec![Value::test_int(2), Value::test_int(2)],
                    ),
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
        let rows: i64 = call.req(engine_state, stack, 0)?;

        let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?;
        let eager: NuDataFrame = lazy
            .into_polars()
            .fetch(rows as usize)
            .map_err(|e| {
                ShellError::GenericError(
                    "Error fetching rows".into(),
                    e.to_string(),
                    Some(call.head),
                    None,
                    Vec::new(),
                )
            })?
            .into();

        Ok(PipelineData::Value(
            NuDataFrame::into_value(eager, call.head),
            None,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(LazyFetch {})])
    }
}
