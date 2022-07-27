use super::super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

use polars::prelude::SeriesMethods;

#[derive(Clone)]
pub struct ValueCount;

impl Command for ValueCount {
    fn name(&self) -> &str {
        "value-counts"
    }

    fn usage(&self) -> &str {
        "Returns a dataframe with the counts for unique values in series"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Calculates value counts",
            example: "[5 5 5 5 6 6] | into df | value-counts",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "0".to_string(),
                        vec![Value::test_int(5), Value::test_int(6)],
                    ),
                    Column::new(
                        "counts".to_string(),
                        vec![Value::test_int(4), Value::test_int(2)],
                    ),
                ])
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

    let res = series.value_counts(false, false).map_err(|e| {
        ShellError::GenericError(
            "Error calculating value counts values".into(),
            e.to_string(),
            Some(call.head),
            Some("The str-slice command can only be used with string columns".into()),
            Vec::new(),
        )
    })?;

    Ok(PipelineData::Value(
        NuDataFrame::dataframe_into_value(res, call.head),
        None,
    ))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ValueCount {})])
    }
}
