use crate::{
    PolarsPlugin,
    values::{
        Column, CustomValueSupport, NuDataFrame, NuExpression, NuLazyFrame, NuSchema,
        PolarsPluginObject, PolarsPluginType, cant_convert_err,
    },
};
use chrono::DateTime;
use std::sync::Arc;

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value, record,
};
use polars::prelude::{DataType, Field, IntoSeries, Schema, StringMethods, StrptimeOptions, col};

#[derive(Clone)]
pub struct AsDate;

impl PluginCommand for AsDate {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars as-date"
    }

    fn description(&self) -> &str {
        r#"Converts string to date."#
    }

    fn extra_description(&self) -> &str {
        r#"Format example:
        "%Y-%m-%d"    => 2021-12-31
        "%d-%m-%Y"    => 31-12-2021
        "%Y%m%d"      => 2021319 (2021-03-19)"#
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("format", SyntaxShape::String, "formatting date string")
            .switch("not-exact", "the format string may be contained in the date (e.g. foo-2021-01-01-bar could match 2021-01-01)", Some('n'))
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
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Converts string to date",
                example: r#"["2021-12-30" "2021-12-31"] | polars into-df | polars as-date "%Y-%m-%d""#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "date".to_string(),
                            vec![
                                // Nushell's Value::date only maps to DataType::Datetime and not DataType::Date
                                // We therefore force the type to be DataType::Date in the schema
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2021-12-30 00:00:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2021-12-31 00:00:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                            ],
                        )],
                        Some(NuSchema::new(Arc::new(Schema::from_iter(vec![
                            Field::new("date".into(), DataType::Date),
                        ])))),
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Converts string to date",
                example: r#"["2021-12-30" "2021-12-31 21:00:00"] | polars into-df | polars as-date "%Y-%m-%d" --not-exact"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "date".to_string(),
                            vec![
                                // Nushell's Value::date only maps to DataType::Datetime and not DataType::Date
                                // We therefore force the type to be DataType::Date in the schema
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2021-12-30 00:00:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2021-12-31 00:00:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                            ],
                        )],
                        Some(NuSchema::new(Arc::new(Schema::from_iter(vec![
                            Field::new("date".into(), DataType::Date),
                        ])))),
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Converts string to date in an expression",
                example: r#"["2021-12-30" "2021-12-31 21:00:00"] | polars into-lazy | polars select (polars col 0 | polars as-date "%Y-%m-%d" --not-exact)"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "date".to_string(),
                            vec![
                                // Nushell's Value::date only maps to DataType::Datetime and not DataType::Date
                                // We therefore force the type to be DataType::Date in the schema
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2021-12-30 00:00:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2021-12-31 00:00:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                            ],
                        )],
                        Some(NuSchema::new(Arc::new(Schema::from_iter(vec![
                            Field::new("date".into(), DataType::Date),
                        ])))),
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Output is of date type",
                example: r#"["2021-12-30" "2021-12-31 21:00:00"] | polars into-df | polars as-date "%Y-%m-%d" --not-exact | polars schema"#,
                result: Some(Value::record(
                    record! {
                        "date" => Value::string("date", Span::test_data()),
                    },
                    Span::test_data(),
                )),
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
    let value = input.into_value(call.head)?;

    let options = StrptimeOptions {
        format: Some(format.into()),
        strict: true,
        exact: !not_exact,
        cache: Default::default(),
    };

    match PolarsPluginObject::try_from_value(plugin, &value)? {
        PolarsPluginObject::NuLazyFrame(lazy) => command_lazy(plugin, engine, call, lazy, options),
        PolarsPluginObject::NuDataFrame(df) => command_eager(plugin, engine, call, df, options),
        PolarsPluginObject::NuExpression(expr) => {
            let res: NuExpression = expr.into_polars().str().to_date(options).into();
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
) -> Result<PipelineData, ShellError> {
    NuLazyFrame::new(
        false,
        lazy.to_polars().select([col("*").str().to_date(options)]),
    )
    .to_pipeline_data(plugin, engine, call.head)
}

fn command_eager(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
    options: StrptimeOptions,
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

    res.rename("date".into());

    let df = NuDataFrame::try_from_series_vec(vec![res], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&AsDate)
    }
}
