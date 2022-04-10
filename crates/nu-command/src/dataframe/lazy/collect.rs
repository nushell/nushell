use super::super::values::{NuDataFrame, NuLazyFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature,
};

#[derive(Clone)]
pub struct LazyCollect;

impl Command for LazyCollect {
    fn name(&self) -> &str {
        "dfl collect"
    }

    fn usage(&self) -> &str {
        "Collect lazy dataframe into dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("dataframe".into()))
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
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?;
        let eager = lazy.collect(call.head)?;

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
//        test_dataframe(vec![Box::new(LazyCollect {})])
//    }
//}
