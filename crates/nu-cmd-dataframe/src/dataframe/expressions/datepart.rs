use super::super::values::NuExpression;

use crate::dataframe::values::{Column, NuDataFrame};
use chrono::{DateTime, FixedOffset};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct ExprDatePart;

impl Command for ExprDatePart {
    fn name(&self) -> &str {
        "dfrexp datepart"
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
                example: r#"[["2021-12-30T01:02:03.123456789"]] | dfr into-df | dfr as-datetime "%Y-%m-%dT%H:%M:%S.%9f" | dfr with-column [(dfrexp col datetime | dfrexp datepart year | dfrexp as datetime_year )]"#,
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new("datetime".to_string(), vec![Value::test_date(dt)]),
                        Column::new("datetime_year".to_string(), vec![Value::test_int(2021)]),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Creates an expression to capture multiple date parts",
                example: r#"[["2021-12-30T01:02:03.123456789"]] | dfr into-df | dfr as-datetime "%Y-%m-%dT%H:%M:%S.%8f" |
                dfr with-column [ (dfrexp col datetime | dfrexp datepart year | dfrexp as datetime_year ),
                (dfrexp col datetime | dfrexp datepart month | dfrexp as datetime_month ),
                (dfrexp col datetime | dfrexp datepart day | dfrexp as datetime_day ),
                (dfrexp col datetime | dfrexp datepart hour | dfrexp as datetime_hour ),
                (dfrexp col datetime | dfrexp datepart minute | dfrexp as datetime_minute ),
                (dfrexp col datetime | dfrexp datepart second | dfrexp as datetime_second ),
                (dfrexp col datetime | dfrexp datepart nanosecond | dfrexp as datetime_ns ) ]"#,
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new("datetime".to_string(), vec![Value::test_date(dt)]),
                        Column::new("datetime_year".to_string(), vec![Value::test_int(2021)]),
                        Column::new("datetime_month".to_string(), vec![Value::test_int(12)]),
                        Column::new("datetime_day".to_string(), vec![Value::test_int(30)]),
                        Column::new("datetime_hour".to_string(), vec![Value::test_int(1)]),
                        Column::new("datetime_minute".to_string(), vec![Value::test_int(2)]),
                        Column::new("datetime_second".to_string(), vec![Value::test_int(3)]),
                        Column::new("datetime_ns".to_string(), vec![Value::test_int(123456789)]),
                    ])
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
                return Err(ShellError::UnsupportedInput(
                    format!("{} is not a valid datepart, expected one of year, month, day, hour, minute, second, millisecond, microsecond, nanosecond", part.item),
                    "value originates from here".to_string(),
                    call.head,
                    part.span,
                ));
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
