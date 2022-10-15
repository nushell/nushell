use super::super::super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};
use polars::prelude::IntoSeries;

#[derive(Clone)]
pub struct ArgUnique;

impl Command for ArgUnique {
    fn name(&self) -> &str {
        "arg-unique"
    }

    fn usage(&self) -> &str {
        "Returns indexes for unique values"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["argunique", "distinct", "noduplicate", "unrepeated"]
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns indexes for unique values",
            example: "[1 2 2 3 3] | into df | arg-unique",
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "arg_unique".to_string(),
                    vec![Value::test_int(0), Value::test_int(1), Value::test_int(3)],
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

    let mut res = df
        .as_series(call.head)?
        .arg_unique()
        .map_err(|e| {
            ShellError::GenericError(
                "Error extracting unique values".into(),
                e.to_string(),
                Some(call.head),
                None,
                Vec::new(),
            )
        })?
        .into_series();
    res.rename("arg_unique");

    NuDataFrame::try_from_series(vec![res], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ArgUnique {})])
    }
}
