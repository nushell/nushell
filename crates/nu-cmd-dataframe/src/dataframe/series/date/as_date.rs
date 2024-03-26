use crate::dataframe::values::NuDataFrame;
use nu_engine::command_prelude::*;

use polars::prelude::{IntoSeries, StringMethods};

#[derive(Clone)]
pub struct AsDate;

impl Command for AsDate {
    fn name(&self) -> &str {
        "dfr as-date"
    }

    fn usage(&self) -> &str {
        r#"Converts string to date."#
    }

    fn extra_usage(&self) -> &str {
        r#"Format example:
        "%Y-%m-%d"    => 2021-12-31
        "%d-%m-%Y"    => 31-12-2021
        "%Y%m%d"      => 2021319 (2021-03-19)"#
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("format", SyntaxShape::String, "formatting date string")
            .switch("not-exact", "the format string may be contained in the date (e.g. foo-2021-01-01-bar could match 2021-01-01)", Some('n'))
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Converts string to date",
            example: r#"["2021-12-30" "2021-12-31"] | dfr into-df | dfr as-datetime "%Y-%m-%d""#,
            result: None,
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
    let format: String = call.req(engine_state, stack, 0)?;
    let not_exact = call.has_flag(engine_state, stack, "not-exact")?;

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;
    let series = df.as_series(call.head)?;
    let casted = series.str().map_err(|e| ShellError::GenericError {
        error: "Error casting to string".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let res = if not_exact {
        casted.as_date_not_exact(Some(format.as_str()))
    } else {
        casted.as_date(Some(format.as_str()), false)
    };

    let mut res = res
        .map_err(|e| ShellError::GenericError {
            error: "Error creating datetime".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?
        .into_series();

    res.rename("date");

    NuDataFrame::try_from_series(vec![res], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}
