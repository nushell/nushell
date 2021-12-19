use super::super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Value,
};

#[derive(Clone)]
pub struct NUnique;

impl Command for NUnique {
    fn name(&self) -> &str {
        "dfr count-unique"
    }

    fn usage(&self) -> &str {
        "Counts unique values"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Counts unique values",
            example: "[1 1 2 2 3 3 4] | dfr to-df | dfr count-unique",
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "count_unique".to_string(),
                    vec![Value::test_int(4)],
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

    let res = df.as_series(call.head)?.n_unique().map_err(|e| {
        ShellError::SpannedLabeledError(
            "Error counting unique values".into(),
            e.to_string(),
            call.head,
        )
    })?;

    let value = Value::Int {
        val: res as i64,
        span: call.head,
    };

    NuDataFrame::try_from_columns(vec![Column::new("count_unique".to_string(), vec![value])])
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(NUnique {})])
    }
}
