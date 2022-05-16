use crate::dataframe::values::NuLazyFrame;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape,
};
use polars::prelude::QuantileInterpolOptions;

#[derive(Clone)]
pub struct LazyQuantile;

impl Command for LazyQuantile {
    fn name(&self) -> &str {
        "dfr quantile"
    }

    fn usage(&self) -> &str {
        "Aggregates the columns to the selected quantile"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "quantile",
                SyntaxShape::Number,
                "quantile value for quantile operation",
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
        let quantile: f64 = call.req(engine_state, stack, 0)?;

        let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?.into_polars();
        let lazy: NuLazyFrame = lazy
            .quantile(quantile, QuantileInterpolOptions::default())
            .into();

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
//        test_dataframe(vec![Box::new(LazyQuantile {})])
//    }
//}
