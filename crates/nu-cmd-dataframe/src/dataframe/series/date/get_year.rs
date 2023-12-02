use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};
use polars::prelude::{DatetimeMethods, IntoSeries};

use super::super::super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct GetYear;

impl Command for GetYear {
    fn name(&self) -> &str {
        "dfr get-year"
    }

    fn usage(&self) -> &str {
        "Gets year from date."
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
            description: "Returns year from a date",
            example: r#"let dt = ('2020-08-04T16:39:18+00:00' | into datetime --timezone 'UTC');
    let df = ([$dt $dt] | dfr into-df);
    $df | dfr get-year"#,
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "0".to_string(),
                    vec![Value::test_int(2020), Value::test_int(2020)],
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
    let series = df.as_series(call.head)?;

    let casted = series.datetime().map_err(|e| {
        ShellError::GenericError(
            "Error casting to datetime type".into(),
            e.to_string(),
            Some(call.head),
            None,
            Vec::new(),
        )
    })?;

    let res = casted.year().into_series();

    NuDataFrame::try_from_series(vec![res], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(explore_refactor_IntoDatetime)]
mod test {
    use super::{
        super::super::super::{super::IntoDatetime, test_dataframe::test_dataframe},
        *,
    };

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(GetYear {}), Box::new(IntoDatetime {})])
    }
}
