use super::super::values::{Column, NuDataFrame};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Shift;

impl Command for Shift {
    fn name(&self) -> &str {
        "dfr shift"
    }

    fn usage(&self) -> &str {
        "Shifts the values by a given period"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("period", SyntaxShape::Int, "shift period")
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Shifts the values by a given period",
            example: "[1 2 2 3 3] | dfr to-df | dfr shift 2 | dfr drop-nulls",
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "0".to_string(),
                    vec![Value::test_int(1), Value::test_int(2), Value::test_int(2)],
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
    let period: i64 = call.req(engine_state, stack, 0)?;

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;
    let series = df.as_series(call.head)?.shift(period);

    NuDataFrame::try_from_series(vec![series], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::super::super::DropNulls;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(Shift {}), Box::new(DropNulls {})])
    }
}
