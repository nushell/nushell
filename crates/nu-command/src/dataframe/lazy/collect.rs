use crate::dataframe::values::{Column, NuDataFrame};

use super::super::values::NuLazyFrame;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Value,
};

#[derive(Clone)]
pub struct LazyCollect;

impl Command for LazyCollect {
    fn name(&self) -> &str {
        "dfr collect"
    }

    fn usage(&self) -> &str {
        "Collect lazy dataframe into eager dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "drop duplicates",
            example: "[[a b]; [1 2] [3 4]] | dfr to-lazy | dfr collect",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new("a".to_string(), vec![Value::Int(1), Value::Int(3)]),
                    Column::new("b".to_string(), vec![Value::Int(2), Value::Int(4)]),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
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
        let value = Value::CustomValue {
            val: Box::new(eager),
            span: call.head,
        };

        Ok(PipelineData::Value(value, None))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(LazyCollect {})])
    }
}
