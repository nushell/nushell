use super::super::values::NuLazyFrame;
use crate::dataframe::values::NuDataFrame;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape,
};

#[derive(Clone)]
pub struct LazyFetch;

impl Command for LazyFetch {
    fn name(&self) -> &str {
        "dfr fetch"
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
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "",
            example: "",
            result: None,
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

//#[cfg(test)]
//mod test {
//    use super::super::super::test_dataframe::test_dataframe;
//    use super::*;
//
//    #[test]
//    fn test_examples() {
//        test_dataframe(vec![Box::new(LazyFetch {})])
//    }
//}
