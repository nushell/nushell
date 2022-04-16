use super::super::values::NuLazyFrame;

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature,
};
use polars::prelude::LazyFrame;

#[derive(Clone)]
pub struct LazyReverse;

impl Command for LazyReverse {
    fn name(&self) -> &str {
        "dfl reverse"
    }

    fn usage(&self) -> &str {
        "Reverts a given lazy dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Takes a dictionary and creates a lazy dataframe",
            example: "[[a b];[1 2] [3 4]] | dfr to-df | dfl to-lazy",
            result: None,
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?;
        let lazy = lazy.apply(LazyFrame::reverse);

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
//        test_dataframe(vec![Box::new(LazyReverse {})])
//    }
//}
