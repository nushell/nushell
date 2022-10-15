use super::super::super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};
use polars::prelude::{IntoSeries, SortOptions};

#[derive(Clone)]
pub struct ArgSort;

impl Command for ArgSort {
    fn name(&self) -> &str {
        "arg-sort"
    }

    fn usage(&self) -> &str {
        "Returns indexes for a sorted series"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["argsort", "order", "arrange"]
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("reverse", "reverse order", Some('r'))
            .switch("nulls-last", "nulls ordered last", Some('n'))
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Returns indexes for a sorted series",
                example: "[1 2 2 3 3] | into df | arg-sort",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "arg_sort".to_string(),
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
                example: "[1 2 2 3 3] | into df | arg-sort -r",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "arg_sort".to_string(),
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

    let sort_options = SortOptions {
        descending: call.has_flag("reverse"),
        nulls_last: call.has_flag("nulls-last"),
    };

    let mut res = df.as_series(call.head)?.argsort(sort_options).into_series();
    res.rename("arg_sort");

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
