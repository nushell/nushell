use super::super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct NNull;

impl Command for NNull {
    fn name(&self) -> &str {
        "dfr count-null"
    }

    fn usage(&self) -> &str {
        "Counts null values."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Counts null values",
            example: r#"let s = ([1 1 0 0 3 3 4] | dfr into-df);
    ($s / $s) | dfr count-null"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "count_null".to_string(),
                        vec![Value::test_int(2)],
                    )],
                    None,
                )
                .expect("simple df for test should not fail")
                .base_value(Span::test_data())
                .expect("rendering base value should not fail"),
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

    let res = df.as_series(call.head)?.null_count();
    let value = Value::int(res as i64, call.head);

    NuDataFrame::try_from_columns(
        vec![Column::new("count_null".to_string(), vec![value])],
        None,
    )
    .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(NNull {})])
    }
}
