use chrono::NaiveDate;
use nu_plugin::PluginCommand;
use nu_protocol::{
    Example, LabeledError, ShellError, Signature, Span, SyntaxShape, Type,
    shell_error::generic::GenericError,
};
use polars::{
    df,
    frame::DataFrame,
    prelude::{Expr, NamedFrom, date_ranges},
    series::Series,
    time::{ClosedWindow, Duration},
};
use polars_lazy::frame::IntoLazy;

use crate::{
    PolarsPlugin,
    command::datetime::{str_to_closed_window, value_to_duration},
    values::{CustomValueSupport, NuDataFrame, NuExpression, PolarsPluginType},
};

pub struct DateRange;

impl PluginCommand for DateRange {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars date-ranges"
    }

    fn description(&self) -> &str {
        "Create a column of date ranges."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(self.name())
            .named(
                "start",
                SyntaxShape::Any,
                "Start of the date range expression. This supports a string, date, or datetime expression.",
                Some('s'),
            )
            .named(
                "end",
                SyntaxShape::Any,
                "End of the date range expression. This supports a string, date, or datetime expression.",
                Some('e'),
            )
            .named(
                "interval",
                SyntaxShape::Any,
                "Interval of the date range expression. This supports a string or duration expression.",
                Some('i'),
            )
            .named(
                "closed",
                SyntaxShape::String,
                "Closed window of the date range expression. This supports 'left', 'right', 'both', or 'none'.",
                Some('c'),
            )
            .switch(
                "eager",
                "Eagerly collect the date range expression into a dataframe.",
                None,
            )
            .input_output_types(vec![
                (Type::Any, PolarsPluginType::NuExpression.into()),
                (Type::Any, PolarsPluginType::NuDataFrame.into()),
            ])
            .category(nu_protocol::Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        let date = |y, m, d| NaiveDate::from_ymd_opt(y, m, d).expect("valid date in example");

        vec![
            Example {
                description: "Eagerly create a column of dates from a start, end, and interval",
                example: "polars date-ranges --start 2022-01-01 --end 2022-01-05 --interval 1d --eager",
                result: Some(
                    NuDataFrame::new(
                        false,
                        df!("literal" => vec![Series::new(
                            "".into(),
                            vec![
                                date(2022, 1, 1),
                                date(2022, 1, 2),
                                date(2022, 1, 3),
                                date(2022, 1, 4),
                                date(2022, 1, 5),
                            ],
                        )])
                        .expect("date range example dataframe should build"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Create a column of per-row date ranges from start and end date columns",
                example: r#"[[start end]; [2022-01-01 2022-01-03] [2022-01-02 2022-01-03]]
 | polars into-df
 | polars select (polars date-ranges --start (polars col start) --end (polars col end) --interval 1d | polars as date_range)"#,
                result: Some(
                    NuDataFrame::new(
                        false,
                        df!("date_range" => vec![
                            Series::new(
                                "".into(),
                                vec![date(2022, 1, 1), date(2022, 1, 2), date(2022, 1, 3)],
                            ),
                            Series::new("".into(), vec![date(2022, 1, 2), date(2022, 1, 3)]),
                        ])
                        .expect("date range example dataframe should build"),
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

    let eager = call.has_flag("eager")?;

    let range = date_ranges(
        start,
        end,
        interval,
        None, // todo - num samples is not yet implemented in polars
        closed_window,
    )
    .map_err(|e| {
        ShellError::Generic(
            GenericError::new(
                "Failed to create date range",
                format!("Failed to create date range: {}", e),
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
                        "Failed to collect date range",
                        format!("Failed to collect date range: {}", e),
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
        test_polars_plugin_command(&DateRange)
    }
}
