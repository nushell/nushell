use super::super::super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Value,
};
use polars::prelude::IntoSeries;

#[derive(Clone)]
pub struct ArgSort;

impl Command for ArgSort {
    fn name(&self) -> &str {
        "dfr arg-sort"
    }

    fn usage(&self) -> &str {
        "Returns indexes for a sorted series"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("reverse", "reverse order", Some('r'))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Returns indexes for a sorted series",
                example: "[1 2 2 3 3] | dfr to-df | dfr arg-sort",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "arg-sort".to_string(),
                        vec![
                            Value::test_int(0),
                            Value::test_int(1),
                            Value::test_int(2),
                            Value::test_int(3),
                            Value::test_int(4),
                        ],
                    )])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Returns indexes for a sorted series",
                example: "[1 2 2 3 3] | dfr to-df | dfr arg-sort -r",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "arg-sort".to_string(),
                        vec![
                            Value::test_int(3),
                            Value::test_int(4),
                            Value::test_int(1),
                            Value::test_int(2),
                            Value::test_int(0),
                        ],
                    )])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        command(engine_state, stack, call, input)
    }
}

fn command(
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let mut res = df
        .as_series(call.head)?
        .argsort(call.has_flag("reverse"))
        .into_series();
    res.rename("arg-sort");

    NuDataFrame::try_from_series(vec![res], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ArgSort {})])
    }
}
