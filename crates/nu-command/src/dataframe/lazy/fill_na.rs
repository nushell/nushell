use crate::dataframe::values::{NuExpression, NuLazyFrame};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct LazyFillNA;

impl Command for LazyFillNA {
    fn name(&self) -> &str {
        "dfr fill-na"
    }

    fn usage(&self) -> &str {
        "Replaces NA values with the given expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "fill",
                SyntaxShape::Any,
                "Expression to use to fill the NAN values",
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
        let fill: Value = call.req(engine_state, stack, 0)?;

        let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?.into_polars();
        let expr = NuExpression::try_from_value(fill)?.into_polars();
        let lazy: NuLazyFrame = lazy.fill_nan(expr).into();

        Ok(PipelineData::Value(lazy.into_value(call.head), None))
    }
}

//#[cfg(test)]
//mod test {
//    use super::super::super::test_dataframe::test_dataframe;
//    use super::*;
//
//    #[test]
//    fn test_examples() {
//        test_dataframe(vec![Box::new(LazyFillNA {})])
//    }
//}
