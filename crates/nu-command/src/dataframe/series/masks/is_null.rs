use super::super::super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Value,
};
use polars::prelude::IntoSeries;

#[derive(Clone)]
pub struct IsNull;

impl Command for IsNull {
    fn name(&self) -> &str {
        "dfr is-null"
    }

    fn usage(&self) -> &str {
        "Creates mask where value is null"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create mask where values are null",
            example: r#"let s = ([5 6 0 8] | dfr to-df);
    let res = ($s / $s);
    $res | dfr is-null"#,
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "is-null".to_string(),
                    vec![
                        Value::test_bool(false),
                        Value::test_bool(false),
                        Value::test_bool(true),
                        Value::test_bool(false),
                    ],
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

    let mut res = df.as_series(call.head)?.is_null();
    res.rename("is-null");

    NuDataFrame::try_from_series(vec![res.into_series()], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(IsNull {})])
    }
}
