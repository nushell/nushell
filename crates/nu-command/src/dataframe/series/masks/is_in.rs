use super::super::super::values::{Column, NuDataFrame};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use polars::prelude::IntoSeries;

#[derive(Clone)]
pub struct IsIn;

impl Command for IsIn {
    fn name(&self) -> &str {
        "is-in"
    }

    fn usage(&self) -> &str {
        "Checks if elements from a series are contained in right series"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("other", SyntaxShape::Any, "right series")
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Checks if elements from a series are contained in right series",
            example: r#"let other = ([1 3 6] | into df);
    [5 6 6 6 8 8 8] | into df | is-in $other"#,
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "is_in".to_string(),
                    vec![
                        Value::test_bool(false),
                        Value::test_bool(true),
                        Value::test_bool(true),
                        Value::test_bool(true),
                        Value::test_bool(false),
                        Value::test_bool(false),
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
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let other_value: Value = call.req(engine_state, stack, 0)?;
    let other_span = other_value.span()?;
    let other_df = NuDataFrame::try_from_value(other_value)?;
    let other = other_df.as_series(other_span)?;

    let mut res = df
        .as_series(call.head)?
        .is_in(&other)
        .map_err(|e| {
            ShellError::GenericError(
                "Error finding in other".into(),
                e.to_string(),
                Some(call.head),
                None,
                Vec::new(),
            )
        })?
        .into_series();

    res.rename("is_in");

    NuDataFrame::try_from_series(vec![res.into_series()], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(IsIn {})])
    }
}
