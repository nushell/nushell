use super::super::super::values::{Column, NuDataFrame};

use chrono::DateTime;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use polars::prelude::{IntoSeries, TimeUnit, Utf8Methods};

#[derive(Clone)]
pub struct AsDateTime;

impl Command for AsDateTime {
    fn name(&self) -> &str {
        "as-datetime"
    }

    fn usage(&self) -> &str {
        r#"Converts string to datetime."#
    }

    fn extra_usage(&self) -> &str {
        r#"Format example:
        "%y/%m/%d %H:%M:%S"  => 21/12/31 12:54:98
        "%y-%m-%d %H:%M:%S"  => 2021-12-31 24:58:01
        "%y/%m/%d %H:%M:%S"  => 21/12/31 24:58:01
        "%y%m%d %H:%M:%S"    => 210319 23:58:50
        "%Y/%m/%d %H:%M:%S"  => 2021/12/31 12:54:98
        "%Y-%m-%d %H:%M:%S"  => 2021-12-31 24:58:01
        "%Y/%m/%d %H:%M:%S"  => 2021/12/31 24:58:01
        "%Y%m%d %H:%M:%S"    => 20210319 23:58:50
        "%FT%H:%M:%S"        => 2019-04-18T02:45:55
        "%FT%H:%M:%S.%6f"    => microseconds
        "%FT%H:%M:%S.%9f"    => nanoseconds"#
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("format", SyntaxShape::String, "formatting date time string")
            .switch("not-exact", "the format string may be contained in the date (e.g. foo-2021-01-01-bar could match 2021-01-01)", Some('n'))
                        .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
.category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Converts string to datetime",
            example: r#"["2021-12-30 00:00:00" "2021-12-31 00:00:00"] | into df | as-datetime "%Y-%m-%d %H:%M:%S""#,
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "datetime".to_string(),
                    vec![
                        Value::Date {
                            val: DateTime::parse_from_str(
                                "2021-12-30 00:00:00 +0000",
                                "%Y-%m-%d %H:%M:%S %z",
                            )
                            .expect("date calculation should not fail in test"),
                            span: Span::test_data(),
                        },
                        Value::Date {
                            val: DateTime::parse_from_str(
                                "2021-12-31 00:00:00 +0000",
                                "%Y-%m-%d %H:%M:%S %z",
                            )
                            .expect("date calculation should not fail in test"),
                            span: Span::test_data(),
                        },
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
    let format: String = call.req(engine_state, stack, 0)?;
    let not_exact = call.has_flag("not-exact");

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;
    let series = df.as_series(call.head)?;
    let casted = series.utf8().map_err(|e| {
        ShellError::GenericError(
            "Error casting to string".into(),
            e.to_string(),
            Some(call.head),
            None,
            Vec::new(),
        )
    })?;

    let res = if not_exact {
        casted.as_datetime_not_exact(Some(format.as_str()), TimeUnit::Milliseconds)
    } else {
        casted.as_datetime(Some(format.as_str()), TimeUnit::Milliseconds)
    };

    let mut res = res
        .map_err(|e| {
            ShellError::GenericError(
                "Error creating datetime".into(),
                e.to_string(),
                Some(call.head),
                None,
                Vec::new(),
            )
        })?
        .into_series();

    res.rename("datetime");
    NuDataFrame::try_from_series(vec![res], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(AsDateTime {})])
    }
}
