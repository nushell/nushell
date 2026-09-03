use chrono::NaiveDate;
use nu_plugin::PluginCommand;
use nu_protocol::{
    Example, LabeledError, ShellError, Signature, Span, Spanned, SyntaxShape, Type,
    shell_error::generic::GenericError,
};
use polars::{
    df,
    frame::DataFrame,
    prelude::{DataType, Duration, Expr, NamedFrom, TimeUnit, TimeZone},
    series::Series,
    time::ClosedWindow,
};
use polars_lazy::frame::IntoLazy;
use polars_plan::dsl::functions::datetime_ranges;

use crate::{
    PolarsPlugin,
    command::datetime::{str_to_closed_window, timezone_from_str, timezone_utc, value_to_duration},
    dataframe::values::str_to_time_unit,
    values::{CustomValueSupport, NuDataFrame, NuExpression, PolarsPluginType},
};

pub struct DatetimeRanges;

impl PluginCommand for DatetimeRanges {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars datetime-ranges"
    }

    fn description(&self) -> &str {
        "Create a column of datetime ranges."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(self.name())
            .named(
                "start",
                SyntaxShape::Any,
                "Start of the datetime range expression. This supports a string, date, or datetime expression.",
                Some('s'),
            )
            .named(
                "end",
                SyntaxShape::Any,
                "End of the datetime range expression. This supports a string, date, or datetime expression.",
                Some('e'),
            )
            .named(
                "interval",
                SyntaxShape::Any,
                "Interval of the datetime range expression. This supports a string or duration expression.",
                Some('i'),
            )
            .named(
                "closed",
                SyntaxShape::String,
                "Closed window of the datetime range expression. This supports 'left', 'right', 'both', or 'none'.",
                Some('c'),
            )
            .named(
                "time-unit",
                SyntaxShape::String,
                "Time unit for the output datetime. One of: ns, us, ms.",
                None,
            )
            .named(
                "time-zone",
                SyntaxShape::String,
                "Time zone for the output datetime. E.g. 'UTC', 'America/New_York'.",
                None,
            )
            .switch(
                "eager",
                "Eagerly collect the datetime range expression into a dataframe.",
                None,
            )
            .input_output_types(vec![
                (Type::Any, PolarsPluginType::NuExpression.into()),
                (Type::Any, PolarsPluginType::NuDataFrame.into()),
            ])
            .category(nu_protocol::Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        let dt = |y, m, d| {
            NaiveDate::from_ymd_opt(y, m, d)
                .expect("valid date in example")
                .and_hms_opt(0, 0, 0)
                .expect("valid time in example")
        };
        let dt_series_naive = |vals: Vec<chrono::NaiveDateTime>| {
            Series::new("".into(), vals)
                .cast(&DataType::Datetime(TimeUnit::Nanoseconds, None))
                .expect("datetime ranges example series should cast")
        };
        let dt_series_utc = |vals: Vec<chrono::NaiveDateTime>| {
            Series::new("".into(), vals)
                .cast(&DataType::Datetime(
                    TimeUnit::Nanoseconds,
                    Some(timezone_utc()),
                ))
                .expect("datetime ranges example series should cast")
        };

        vec![
            Example {
                description: "Eagerly create a column of per-row datetime ranges from a start, end, and interval",
                example: "polars datetime-ranges --start 2022-01-01 --end 2022-01-05 --interval 1d --eager",
                result: Some(
                    NuDataFrame::new(
                        false,
                        df!("literal" => &[dt_series_naive(vec![
                            dt(2022, 1, 1),
                            dt(2022, 1, 2),
                            dt(2022, 1, 3),
                            dt(2022, 1, 4),
                            dt(2022, 1, 5),
                        ])])
                        .expect("datetime ranges example dataframe should build"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Create a column of per-row datetime ranges from start and end date columns",
                example: r#"[[start end]; [2022-01-01 2022-01-03] [2022-01-02 2022-01-03]]
    | polars into-df
    | polars select (polars datetime-ranges --start (polars col start) --end (polars col end) --interval 1d | polars as date_range)"#,
                result: Some(
                    NuDataFrame::new(
                        false,
                        df!("date_range" => &[
                            dt_series_utc(vec![dt(2022, 1, 1), dt(2022, 1, 2), dt(2022, 1, 3)]),
                            dt_series_utc(vec![dt(2022, 1, 2), dt(2022, 1, 3)]),
                        ])
                        .expect("datetime ranges example dataframe should build"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        mut input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::LabeledError> {
        let metadata = input.take_metadata();
        command(plugin, engine, call)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &nu_plugin::EngineInterface,
    call: &nu_plugin::EvaluatedCall,
) -> Result<nu_protocol::PipelineData, ShellError> {
    let start: Option<Expr> = call
        .get_flag_value("start")
        .map(|ref v| NuExpression::try_from_value(plugin, v))
        .transpose()?
        .map(|e| e.into_polars());

    let end: Option<Expr> = call
        .get_flag_value("end")
        .map(|ref v| NuExpression::try_from_value(plugin, v))
        .transpose()?
        .map(|e| e.into_polars());

    let interval: Option<Duration> = call
        .get_flag_value("interval")
        .map(|ref v| value_to_duration(v))
        .transpose()?;

    let closed_window: ClosedWindow = call
        .get_flag_value("closed")
        .map(|ref v| {
            let closed_str = v.as_str()?;
            str_to_closed_window(closed_str, v.span())
        })
        .transpose()?
        .unwrap_or(ClosedWindow::Both);

    let time_unit: Option<TimeUnit> = call
        .get_flag::<Spanned<String>>("time-unit")?
        .map(|s| str_to_time_unit(&s.item, s.span))
        .transpose()?;

    let time_zone: Option<TimeZone> = call
        .get_flag::<Spanned<String>>("time-zone")?
        .map(|s| timezone_from_str(&s.item, Some(s.span)))
        .transpose()?;

    let eager = call.has_flag("eager")?;

    let range = datetime_ranges(
        start,
        end,
        interval,
        None, // todo - num samples is not yet implemented in polars
        closed_window,
        time_unit,
        time_zone,
    )
    .map_err(|e| {
        ShellError::Generic(
            GenericError::new(
                "Failed to create datetime range",
                format!("Failed to create datetime range: {}", e),
                call.head,
            )
            .with_source(e),
        )
    })?;

    if eager {
        let df = DataFrame::empty()
            .lazy()
            .select([range])
            .collect()
            .map_err(|e| {
                ShellError::Generic(
                    GenericError::new(
                        "Failed to collect datetime range",
                        format!("Failed to collect datetime range: {}", e),
                        call.head,
                    )
                    .with_source(e),
                )
            })?;
        NuDataFrame::new(false, df).to_pipeline_data(plugin, engine, call.head)
    } else {
        NuExpression::from(range).to_pipeline_data(plugin, engine, call.head)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&DatetimeRanges)
    }
}
