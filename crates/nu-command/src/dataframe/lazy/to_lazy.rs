use super::super::values::{NuDataFrame, NuLazyFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Type, Value,
};

#[derive(Clone)]
pub struct ToLazyFrame;

impl Command for ToLazyFrame {
    fn name(&self) -> &str {
        "into lazy"
    }

    fn usage(&self) -> &str {
        "Converts a dataframe into a lazy dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_type(Type::Any)
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Takes a dictionary and creates a lazy dataframe",
            example: "[[a b];[1 2] [3 4]] | into lazy",
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
        let value = Value::CustomValue {
            val: Box::new(lazy),
            span: call.head,
        };

        Ok(PipelineData::Value(value, None))
    }
}
