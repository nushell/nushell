use crate::{
    PolarsPlugin,
    values::{
        Column, CustomValueSupport, NuDataFrame, NuExpression, NuLazyFrame, NuSchema,
        PolarsPluginObject, PolarsPluginType, cant_convert_err,
    },
};
use chrono::DateTime;
use polars_plan::plans::DynLiteralValue;
use std::sync::Arc;

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::prelude::{
    DataType, Expr, Field, IntoSeries, LiteralValue, PlSmallStr, Schema, StringMethods,
    StrptimeOptions, TimeUnit, col,
};

#[derive(Clone)]
pub struct AsDateTime;

impl PluginCommand for AsDateTime {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars as-datetime"
    }

    fn description(&self) -> &str {
        r#"Converts string to datetime."#
    }

    fn extra_description(&self) -> &str {
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
            .input_output_types(vec![
                (
                    Type::Custom("dataframe".into()),
                    Type::Custom("dataframe".into()),
                ),
                (
                    Type::Custom("expression".into()),
                    Type::Custom("expression".into()),
                ),
            ])
            .required("format", SyntaxShape::String, "formatting date time string")
            .switch("not-exact", "the format string may be contained in the date (e.g. foo-2021-01-01-bar could match 2021-01-01)", Some('n'))
            .switch("naive", "the input datetimes should be parsed as naive (i.e., not timezone-aware). Ignored if input is an expression.", None)
            .named(
                "ambiguous",
                SyntaxShape::OneOf(vec![SyntaxShape::String, SyntaxShape::Nothing]),
                r#"Determine how to deal with ambiguous datetimes:
                    `raise` (default): raise error
                    `earliest`: use the earliest datetime
                    `latest`: use the latest datetime
                    `null`: set to null
                    Used only when input is a lazyframe or expression and ignored otherwise"#,
                Some('a'),
            )            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Converts string to datetime",
                example: r#"["2021-12-30 00:00:00 -0400" "2021-12-31 00:00:00 -0400"] | polars into-df | polars as-datetime "%Y-%m-%d %H:%M:%S %z""#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "datetime".to_string(),
                            vec![
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2021-12-30 00:00:00 -0400",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2021-12-31 00:00:00 -0400",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Converts string to datetime with high resolutions",
                example: r#"["2021-12-30 00:00:00.123456789" "2021-12-31 00:00:00.123456789"] | polars into-df | polars as-datetime "%Y-%m-%d %H:%M:%S.%9f" --naive"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "datetime".to_string(),
                            vec![
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2021-12-30 00:00:00.123456789 +0000",
                                        "%Y-%m-%d %H:%M:%S.%9f %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2021-12-31 00:00:00.123456789 +0000",
                                        "%Y-%m-%d %H:%M:%S.%9f %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                            ],
                        )],
                        Some(NuSchema::new(Arc::new(Schema::from_iter(vec![
                            Field::new(
                                "datetime".into(),
                                DataType::Datetime(TimeUnit::Nanoseconds, None),
                            ),
                        ])))),
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Converts string to datetime using the `--not-exact` flag even with excessive symbols",
                example: r#"["2021-12-30 00:00:00 GMT+4"] | polars into-df | polars as-datetime "%Y-%m-%d %H:%M:%S" --not-exact --naive"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "datetime".to_string(),
                            vec![Value::date(
                                DateTime::parse_from_str(
                                    "2021-12-30 00:00:00 +0000",
                                    "%Y-%m-%d %H:%M:%S %z",
                                )
                                .expect("date calculation should not fail in test"),
                                Span::test_data(),
                            )],
                        )],
                        Some(NuSchema::new(Arc::new(Schema::from_iter(vec![
                            Field::new(
                                "datetime".into(),
                                DataType::Datetime(TimeUnit::Nanoseconds, None),
                            ),
                        ])))),
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Converts string to datetime using the `--not-exact` flag even with excessive symbols in an expression",
                example: r#"["2025-11-02 00:00:00", "2025-11-02 01:00:00", "2025-11-02 02:00:00", "2025-11-02 03:00:00"] | polars into-df | polars select (polars col 0 | polars as-datetime "%Y-%m-%d %H:%M:%S")"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "datetime".to_string(),
                            vec![
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2025-11-02 00:00:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2025-11-02 01:00:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2025-11-02 02:00:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2025-11-02 03:00:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                            ],
                        )],
                        Some(NuSchema::new(Arc::new(Schema::from_iter(vec![
                            Field::new(
                                "datetime".into(),
                                DataType::Datetime(TimeUnit::Nanoseconds, None),
                            ),
                        ])))),
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
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
        command(plugin, engine, call, input)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let format: String = call.req(0)?;
    let not_exact = call.has_flag("not-exact")?;
    let tz_aware = !call.has_flag("naive")?;

    let value = input.into_value(call.head)?;

    let options = StrptimeOptions {
        format: Some(format.into()),
        strict: true,
        exact: !not_exact,
        cache: Default::default(),
    };

    let ambiguous = match call.get_flag::<Value>("ambiguous")? {
        Some(Value::String { val, internal_span }) => match val.as_str() {
            "raise" | "earliest" | "latest" => Ok(val),
            _ => Err(ShellError::GenericError {
                error: "Invalid argument value".into(),
                msg: "`ambiguous` must be one of raise, earliest, latest, or null".into(),
                span: Some(internal_span),
                help: None,
                inner: vec![],
            }),
        },
        Some(Value::Nothing { .. }) => Ok("null".into()),
        Some(_) => unreachable!("Argument only accepts string or null."),
        None => Ok("raise".into()),
    }
    .map_err(LabeledError::from)?;

    match PolarsPluginObject::try_from_value(plugin, &value)? {
        PolarsPluginObject::NuLazyFrame(lazy) => {
            command_lazy(plugin, engine, call, lazy, options, ambiguous)
        }
        PolarsPluginObject::NuDataFrame(df) => {
            command_eager(plugin, engine, call, df, options, tz_aware)
        }
        PolarsPluginObject::NuExpression(expr) => {
            let res: NuExpression = expr
                .into_polars()
                .str()
                .to_datetime(
                    None,
                    None,
                    options,
                    Expr::Literal(LiteralValue::Dyn(DynLiteralValue::Str(
                        PlSmallStr::from_string(ambiguous),
                    ))),
                )
                .into();
            res.to_pipeline_data(plugin, engine, call.head)
        }
        _ => Err(cant_convert_err(
            &value,
            &[
                PolarsPluginType::NuDataFrame,
                PolarsPluginType::NuLazyFrame,
                PolarsPluginType::NuExpression,
            ],
        )),
    }
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
    options: StrptimeOptions,
    ambiguous: String,
) -> Result<PipelineData, ShellError> {
    NuLazyFrame::new(
        false,
        lazy.to_polars().select([col("*").str().to_datetime(
            None,
            None,
            options,
            Expr::Literal(LiteralValue::Dyn(DynLiteralValue::Str(
                PlSmallStr::from_string(ambiguous),
            ))),
        )]),
    )
    .to_pipeline_data(plugin, engine, call.head)
}

fn command_eager(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
    options: StrptimeOptions,
    tz_aware: bool,
) -> Result<PipelineData, ShellError> {
    let format = if let Some(format) = options.format {
        format.to_string()
    } else {
        unreachable!("`format` will never be None")
    };
    let not_exact = !options.exact;

    let series = df.as_series(call.head)?;
    let casted = series.str().map_err(|e| ShellError::GenericError {
        error: "Error casting to string".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let res = if not_exact {
        casted.as_datetime_not_exact(
            Some(format.as_str()),
            TimeUnit::Nanoseconds,
            tz_aware,
            None,
            &Default::default(),
        )
    } else {
        casted.as_datetime(
            Some(format.as_str()),
            TimeUnit::Nanoseconds,
            false,
            tz_aware,
            None,
            &Default::default(),
        )
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

    res.rename("datetime".into());
    let df = NuDataFrame::try_from_series_vec(vec![res], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command_with_decls;
    use nu_command::IntoDatetime;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command_with_decls(&AsDateTime, vec![Box::new(IntoDatetime)])
    }
}
