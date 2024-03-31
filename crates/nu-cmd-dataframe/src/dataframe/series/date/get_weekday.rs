use crate::dataframe::values::{Column, NuDataFrame};
use nu_engine::command_prelude::*;
use polars::prelude::{DatetimeMethods, IntoSeries};

#[derive(Clone)]
pub struct GetWeekDay;

impl Command for GetWeekDay {
    fn name(&self) -> &str {
        "dfr get-weekday"
    }

    fn usage(&self) -> &str {
        "Gets weekday from date."
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
            description: "Returns weekday from a date",
            example: r#"let dt = ('2020-08-04T16:39:18+00:00' | into datetime --timezone 'UTC');
    let df = ([$dt $dt] | dfr into-df);
    $df | dfr get-weekday"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "0".to_string(),
                        vec![Value::test_int(2), Value::test_int(2)],
                    )],
                    None,
                )
                .expect("simple df for test should not fail")
                .into_value(FutureSpanId::test_data()),
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

    let casted = series.datetime().map_err(|e| ShellError::GenericError {
        error: "Error casting to datetime type".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let res = casted.weekday().into_series();

    NuDataFrame::try_from_series(vec![res], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(explore_refactor_IntoDatetime)]
mod test {
    use super::super::super::super::super::IntoDatetime;
    use super::super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(GetWeekDay {}), Box::new(IntoDatetime {})])
    }
}
