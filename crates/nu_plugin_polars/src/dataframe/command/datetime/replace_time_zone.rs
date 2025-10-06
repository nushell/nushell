use crate::values::{Column, NuDataFrame, NuSchema};
use crate::{
    PolarsPlugin,
    dataframe::values::NuExpression,
    values::{CustomValueSupport, PolarsPluginObject, PolarsPluginType, cant_convert_err},
};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Value,
};

use chrono::DateTime;
use polars::prelude::*;
use polars_plan::plans::DynLiteralValue;

use super::timezone_from_str;

#[derive(Clone)]
pub struct ReplaceTimeZone;

impl PluginCommand for ReplaceTimeZone {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars replace-time-zone"
    }

    fn description(&self) -> &str {
        "Replace the timezone information in a datetime column."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(
                PolarsPluginType::NuExpression.into(),
                PolarsPluginType::NuExpression.into(),
            )])
            .required(
                "time_zone",
                SyntaxShape::String,
                "Timezone for the Datetime Series. Pass `null` to unset time zone.",
            )
            .named(
                "ambiguous",
                SyntaxShape::OneOf(vec![SyntaxShape::String, SyntaxShape::Nothing]),
                r#"Determine how to deal with ambiguous datetimes:
                    `raise` (default): raise error
                    `earliest`: use the earliest datetime
                    `latest`: use the latest datetime
                    `null`: set to null"#,
                Some('a'),
            )
            .named(
                "nonexistent",
                SyntaxShape::OneOf(vec![SyntaxShape::String, SyntaxShape::Nothing]),
                r#"Determine how to deal with non-existent datetimes: raise (default) or null."#,
                Some('n'),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Apply timezone to a naive datetime",
                example: r#"["2021-12-30 00:00:00" "2021-12-31 00:00:00"] | polars into-df
                    | polars as-datetime "%Y-%m-%d %H:%M:%S" --naive
                    | polars select (polars col datetime | polars replace-time-zone "America/New_York")"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "datetime".to_string(),
                            vec![
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2021-12-30 00:00:00 -0500",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2021-12-31 00:00:00 -0500",
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
                                DataType::Datetime(
                                    TimeUnit::Nanoseconds,
                                    TimeZone::opt_try_new(Some("America/New_York"))
                                        .expect("timezone should be valid"),
                                ),
                            ),
                        ])))),
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Apply timezone with ambiguous datetime",
                example: r#"["2025-11-02 00:00:00", "2025-11-02 01:00:00", "2025-11-02 02:00:00", "2025-11-02 03:00:00"]
                    | polars into-df
                    | polars as-datetime "%Y-%m-%d %H:%M:%S" --naive
                    | polars select (polars col datetime | polars replace-time-zone "America/New_York" --ambiguous null)"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "datetime".to_string(),
                            vec![
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2025-11-02 00:00:00 -0400",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::nothing(Span::test_data()),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2025-11-02 02:00:00 -0500",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2025-11-02 03:00:00 -0500",
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
                                DataType::Datetime(
                                    TimeUnit::Nanoseconds,
                                    TimeZone::opt_try_new(Some("America/New_York"))
                                        .expect("timezone should be valid"),
                                ),
                            ),
                        ])))),
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Apply timezone with nonexistent datetime",
                example: r#"["2025-03-09 01:00:00", "2025-03-09 02:00:00", "2025-03-09 03:00:00", "2025-03-09 04:00:00"]
                    | polars into-df
                    | polars as-datetime "%Y-%m-%d %H:%M:%S" --naive
                    | polars select (polars col datetime | polars replace-time-zone "America/New_York" --nonexistent null)"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "datetime".to_string(),
                            vec![
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2025-03-09 01:00:00 -0500",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::nothing(Span::test_data()),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2025-03-09 03:00:00 -0400",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2025-03-09 04:00:00 -0400",
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
                                DataType::Datetime(
                                    TimeUnit::Nanoseconds,
                                    TimeZone::opt_try_new(Some("America/New_York"))
                                        .expect("timezone should be valid"),
                                ),
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
        let value = input.into_value(call.head)?;

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

        let nonexistent = match call.get_flag::<Value>("nonexistent")? {
            Some(Value::String { val, internal_span }) => match val.as_str() {
                "raise" => Ok(NonExistent::Raise),
                _ => Err(ShellError::GenericError {
                    error: "Invalid argument value".into(),
                    msg: "`nonexistent` must be one of raise or null".into(),
                    span: Some(internal_span),
                    help: None,
                    inner: vec![],
                }),
            },
            Some(Value::Nothing { .. }) => Ok(NonExistent::Null),
            Some(_) => unreachable!("Argument only accepts string or null."),
            None => Ok(NonExistent::Raise),
        }
        .map_err(LabeledError::from)?;

        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuExpression(expr) => {
                let time_zone_spanned: Spanned<String> = call.req(0)?;
                let time_zone =
                    timezone_from_str(&time_zone_spanned.item, Some(time_zone_spanned.span))?;
                let expr: NuExpression = expr
                    .into_polars()
                    .dt()
                    .replace_time_zone(
                        Some(time_zone),
                        Expr::Literal(LiteralValue::Dyn(DynLiteralValue::Str(
                            PlSmallStr::from_string(ambiguous),
                        ))),
                        nonexistent,
                    )
                    .into();
                expr.to_pipeline_data(plugin, engine, call.head)
            }
            _ => Err(cant_convert_err(&value, &[PolarsPluginType::NuExpression])),
        }
        .map_err(LabeledError::from)
        .map(|pd| pd.set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::test::test_polars_plugin_command;
    use nu_protocol::ShellError;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ReplaceTimeZone)
    }
}
