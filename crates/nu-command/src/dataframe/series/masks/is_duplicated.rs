use super::super::super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span,
};
use polars::prelude::IntoSeries;

#[derive(Clone)]
pub struct IsDuplicated;

impl Command for IsDuplicated {
    fn name(&self) -> &str {
        "dfr is-duplicated"
    }

    fn usage(&self) -> &str {
        "Creates mask indicating duplicated values"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create mask indicating duplicated values",
            example: "[5 6 6 6 8 8 8] | dfr to-df | dfr is-duplicated",
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "is_duplicated".to_string(),
                    vec![
                        false.into(),
                        true.into(),
                        true.into(),
                        true.into(),
                        true.into(),
                        true.into(),
                        true.into(),
                    ],
                )])
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

    let mut res = df
        .as_series(call.head)?
        .is_duplicated()
        .map_err(|e| {
            ShellError::SpannedLabeledError(
                "Error finding duplicates".into(),
                e.to_string(),
                call.head,
            )
        })?
        .into_series();

    res.rename("is_duplicated");

    NuDataFrame::try_from_series(vec![res.into_series()], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(IsDuplicated {})])
    }
}
