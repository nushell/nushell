use super::super::values::{Column, NuDataFrame};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct NUnique;

impl Command for NUnique {
    fn name(&self) -> &str {
        "n-unique"
    }

    fn usage(&self) -> &str {
        "Counts unique values"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Counts unique values",
            example: "[1 1 2 2 3 3 4] | into df | n-unique",
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
        let df = NuDataFrame::try_from_pipeline(input, call.head)?;
        command(engine_state, stack, call, df)
    }
}

fn command(
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let res = df.as_series(call.head)?.n_unique().map_err(|e| {
        ShellError::GenericError(
            "Error counting unique values".into(),
            e.to_string(),
            Some(call.head),
            None,
            Vec::new(),
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
