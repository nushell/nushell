use crate::dataframe::values::{Column, NuDataFrame, NuExpression};
use chrono::{DateTime, FixedOffset};
use nu_engine::command_prelude::*;

use polars::{
    datatypes::{DataType, TimeUnit},
    prelude::NamedFrom,
    series::Series,
};

#[derive(Clone)]
pub struct ExprDatePart;

impl Command for ExprDatePart {
    fn name(&self) -> &str {
        "dfr datepart"
    }

    fn usage(&self) -> &str {
        "Creates an expression for capturing the specified datepart in a column."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "Datepart name",
                SyntaxShape::String,
                "Part of the date to capture.  Possible values are year, quarter, month, week, weekday, day, hour, minute, second, millisecond, microsecond, nanosecond",
            )
            .input_output_type(
                Type::Custom("expression".into()),
                Type::Custom("expression".into()),
            )
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        let dt = DateTime::<FixedOffset>::parse_from_str(
            "2021-12-30T01:02:03.123456789 +0000",
            "%Y-%m-%dT%H:%M:%S.%9f %z",
        )
        .expect("date calculation should not fail in test");
        vec![
            Example {
                description: "Creates an expression to capture the year date part",
                example: r#"[["2021-12-30T01:02:03.123456789"]] | dfr into-df | dfr as-datetime "%Y-%m-%dT%H:%M:%S.%9f" | dfr with-column [(dfr col datetime | dfr datepart year | dfr as datetime_year )]"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new("datetime".to_string(), vec![Value::test_date(dt)]),
                            Column::new("datetime_year".to_string(), vec![Value::test_int(2021)]),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Creates an expression to capture multiple date parts",
                example: r#"[["2021-12-30T01:02:03.123456789"]] | dfr into-df | dfr as-datetime "%Y-%m-%dT%H:%M:%S.%9f" |
                dfr with-column [ (dfr col datetime | dfr datepart year | dfr as datetime_year ),
                (dfr col datetime | dfr datepart month | dfr as datetime_month ),
                (dfr col datetime | dfr datepart day | dfr as datetime_day ),
                (dfr col datetime | dfr datepart hour | dfr as datetime_hour ),
                (dfr col datetime | dfr datepart minute | dfr as datetime_minute ),
                (dfr col datetime | dfr datepart second | dfr as datetime_second ),
                (dfr col datetime | dfr datepart nanosecond | dfr as datetime_ns ) ]"#,
                result: Some(
                    NuDataFrame::try_from_series(
                        vec![
                            Series::new("datetime", &[dt.timestamp_nanos_opt()])
                                .cast(&DataType::Datetime(TimeUnit::Nanoseconds, None))
                                .expect("Error casting to datetime type"),
                            Series::new("datetime_year", &[2021_i64]), // i32 was coerced to i64
                            Series::new("datetime_month", &[12_i8]),
                            Series::new("datetime_day", &[30_i8]),
                            Series::new("datetime_hour", &[1_i8]),
                            Series::new("datetime_minute", &[2_i8]),
                            Series::new("datetime_second", &[3_i8]),
                            Series::new("datetime_ns", &[123456789_i64]), // i32 was coerced to i64
                        ],
                        Span::test_data(),
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "year",
            "month",
            "week",
            "weekday",
            "quarter",
            "day",
            "hour",
            "minute",
            "second",
            "millisecond",
            "microsecond",
            "nanosecond",
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let part: Spanned<String> = call.req(engine_state, stack, 0)?;

        let expr = NuExpression::try_from_pipeline(input, call.head)?;
        let expr_dt = expr.into_polars().dt();
        let expr = match part.item.as_str() {
            "year" => expr_dt.year(),
            "quarter" => expr_dt.quarter(),
            "month" => expr_dt.month(),
            "week" => expr_dt.week(),
            "day" => expr_dt.day(),
            "hour" => expr_dt.hour(),
            "minute" => expr_dt.minute(),
            "second" => expr_dt.second(),
            "millisecond" => expr_dt.millisecond(),
            "microsecond" => expr_dt.microsecond(),
            "nanosecond" => expr_dt.nanosecond(),
            _ => {
                return Err(ShellError::UnsupportedInput {
                    msg: format!("{} is not a valid datepart, expected one of year, month, day, hour, minute, second, millisecond, microsecond, nanosecond", part.item),
                    input: "value originates from here".to_string(),
                    msg_span: call.head,
                    input_span: part.span,
                });
            }
        }.into();

        Ok(PipelineData::Value(
            NuExpression::into_value(expr, call.head),
            None,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;
    use crate::dataframe::eager::ToNu;
    use crate::dataframe::eager::WithColumn;
    use crate::dataframe::expressions::ExprAlias;
    use crate::dataframe::expressions::ExprCol;
    use crate::dataframe::series::AsDateTime;

    #[test]
    fn test_examples() {
        test_dataframe(vec![
            Box::new(ExprDatePart {}),
            Box::new(ExprCol {}),
            Box::new(ToNu {}),
            Box::new(AsDateTime {}),
            Box::new(WithColumn {}),
            Box::new(ExprAlias {}),
        ])
    }
}
