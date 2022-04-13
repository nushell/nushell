use super::super::values::NuLazyFrame;
use crate::dataframe::values::NuExpression;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use polars::prelude::LazyFrame;

#[derive(Clone)]
pub struct LazyWithColumn;

impl Command for LazyWithColumn {
    fn name(&self) -> &str {
        "dfl with-column"
    }

    fn usage(&self) -> &str {
        "Adds a new column for the dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "Defining expression",
                SyntaxShape::Any,
                "Expression to create the column",
            )
            .category(Category::Custom("dataframe".into()))
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
        let expr: Value = call.req(engine_state, stack, 0)?;
        let expr = NuExpression::try_from_value(expr)?;

        let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?;
        let lazy = lazy.apply_with_expr(expr, LazyFrame::with_column);

        Ok(PipelineData::Value(
            NuLazyFrame::into_value(lazy, call.head),
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
//        test_dataframe(vec![Box::new(LazyWithColumn {})])
//    }
//}
