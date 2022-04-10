use super::super::values::{NuDataFrame, NuLazyFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature,
};

#[derive(Clone)]
pub struct ToLazyFrame;

impl Command for ToLazyFrame {
    fn name(&self) -> &str {
        "dfl to-lazy"
    }

    fn usage(&self) -> &str {
        "Converts a dataframe into a lazy dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("dataframe".into()))
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
        let df = NuDataFrame::try_from_iter(input.into_iter())?;
        let lazy = NuLazyFrame::from_dataframe(df);

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
//        test_dataframe(vec![Box::new(ToLazyFrame {})])
//    }
//}
