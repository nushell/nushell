use super::super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};
use polars::prelude::{IntoSeries, NewChunkedArray, UInt32Chunked};

#[derive(Clone)]
pub struct ArgMax;

impl Command for ArgMax {
    fn name(&self) -> &str {
        "arg-max"
    }

    fn usage(&self) -> &str {
        "Return index for max value in series"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["argmax", "maximum", "most", "largest", "greatest"]
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns index for max value",
            example: "[1 3 2] | into df | arg-max",
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "arg_max".to_string(),
                    vec![Value::test_int(1)],
                )])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        }]
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
    let series = df.as_series(call.head)?;

    let res = series.arg_max();
    let chunked = match res {
        Some(index) => UInt32Chunked::from_slice("arg_max", &[index as u32]),
        None => UInt32Chunked::from_slice("arg_max", &[]),
    };

    let res = chunked.into_series();
    NuDataFrame::try_from_series(vec![res], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ArgMax {})])
    }
}
