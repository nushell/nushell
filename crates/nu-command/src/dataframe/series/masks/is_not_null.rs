use super::super::super::values::{Column, NuDataFrame};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};
use polars::prelude::IntoSeries;

#[derive(Clone)]
pub struct IsNotNull;

impl Command for IsNotNull {
    fn name(&self) -> &str {
        "is-not-null"
    }

    fn usage(&self) -> &str {
        "Creates mask where value is not null"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create mask where values are not null",
            example: r#"let s = ([5 6 0 8] | into df);
    let res = ($s / $s);
    $res | is-not-null"#,
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "is_not_null".to_string(),
                    vec![
                        Value::test_bool(true),
                        Value::test_bool(true),
                        Value::test_bool(false),
                        Value::test_bool(true),
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
    let mut res = df.as_series(call.head)?.is_not_null();
    res.rename("is_not_null");

    NuDataFrame::try_from_series(vec![res.into_series()], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(IsNotNull {})])
    }
}
