use super::super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span,
};

#[derive(Clone)]
pub struct ValueCount;

impl Command for ValueCount {
    fn name(&self) -> &str {
        "dfr value-counts"
    }

    fn usage(&self) -> &str {
        "Returns a dataframe with the counts for unique values in series"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Calculates value counts",
            example: "[5 5 5 5 6 6] | dfr to-df | dfr value-counts",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new("0".to_string(), vec![5.into(), 6.into()]),
                    Column::new("counts".to_string(), vec![4.into(), 2.into()]),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::unknown()),
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

    let res = series.value_counts().map_err(|e| {
        ShellError::SpannedLabeledErrorHelp(
            "Error calculating value counts values".into(),
            e.to_string(),
            call.head,
            "The str-slice command can only be used with string columns".into(),
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
