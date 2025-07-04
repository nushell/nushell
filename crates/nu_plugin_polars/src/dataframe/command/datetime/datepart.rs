use crate::values::NuExpression;
use std::sync::Arc;

use crate::{
    PolarsPlugin,
    dataframe::values::{Column, NuDataFrame, NuSchema},
    values::CustomValueSupport,
};
use chrono::{DateTime, FixedOffset};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};
use polars::{
    datatypes::{DataType, TimeUnit},
    prelude::{Field, NamedFrom, Schema},
    series::Series,
};

#[derive(Clone)]
pub struct ExprDatePart;

impl PluginCommand for ExprDatePart {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars datepart"
    }

    fn description(&self) -> &str {
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
                example: r#"[["2021-12-30T01:02:03.123456789"]] | polars into-df | polars as-datetime "%Y-%m-%dT%H:%M:%S.%9f" --naive | polars with-column [(polars col datetime | polars datepart year | polars as datetime_year )]"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new("datetime".to_string(), vec![Value::test_date(dt)]),
                            Column::new("datetime_year".to_string(), vec![Value::test_int(2021)]),
                        ],
                        Some(NuSchema::new(Arc::new(Schema::from_iter(vec![
                            Field::new(
                                "datetime".into(),
                                DataType::Datetime(TimeUnit::Nanoseconds, None),
                            ),
                            Field::new("datetime_year".into(), DataType::Int64),
                        ])))),
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Creates an expression to capture multiple date parts",
                example: r#"[["2021-12-30T01:02:03.123456789"]] | polars into-df | polars as-datetime "%Y-%m-%dT%H:%M:%S.%9f" --naive |
                polars with-column [ (polars col datetime | polars datepart year | polars as datetime_year ),
                (polars col datetime | polars datepart month | polars as datetime_month ),
                (polars col datetime | polars datepart day | polars as datetime_day ),
                (polars col datetime | polars datepart hour | polars as datetime_hour ),
                (polars col datetime | polars datepart minute | polars as datetime_minute ),
                (polars col datetime | polars datepart second | polars as datetime_second ),
                (polars col datetime | polars datepart nanosecond | polars as datetime_ns ) ]"#,
                result: Some(
                    NuDataFrame::try_from_series_vec(
                        vec![
                            Series::new("datetime".into(), &[dt.timestamp_nanos_opt()])
                                .cast(&DataType::Datetime(TimeUnit::Nanoseconds, None))
                                .expect("Error casting to datetime type"),
                            Series::new("datetime_year".into(), &[2021_i64]), // i32 was coerced to i64
                            Series::new("datetime_month".into(), &[12_i8]),
                            Series::new("datetime_day".into(), &[30_i8]),
                            Series::new("datetime_hour".into(), &[1_i8]),
                            Series::new("datetime_minute".into(), &[2_i8]),
                            Series::new("datetime_second".into(), &[3_i8]),
                            Series::new("datetime_ns".into(), &[123456789_i64]), // i32 was coerced to i64
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
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let part: Spanned<String> = call.req(0)?;

        let expr = NuExpression::try_from_pipeline(plugin, input, call.head)?;
        let expr_dt = expr.into_polars().dt();
        let expr: NuExpression  = match part.item.as_str() {
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
                return Err(LabeledError::from(ShellError::UnsupportedInput {
                    msg: format!("{} is not a valid datepart, expected one of year, month, day, hour, minute, second, millisecond, microsecond, nanosecond", part.item),
                    input: "value originates from here".to_string(),
                    msg_span: call.head,
                    input_span: part.span,
                }))
            }
        }.into();
        expr.to_pipeline_data(plugin, engine, call.head)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ExprDatePart)
    }
}
