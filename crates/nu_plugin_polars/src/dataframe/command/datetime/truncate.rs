use crate::{
    PolarsPlugin,
    values::{
        Column, CustomValueSupport, NuDataFrame, NuExpression, NuSchema, PolarsPluginObject,
        PolarsPluginType, cant_convert_err,
    },
};
use std::sync::Arc;

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

use chrono::DateTime;
use polars::prelude::{DataType, Expr, Field, LiteralValue, PlSmallStr, Schema, TimeUnit};
use polars_plan::plans::DynLiteralValue;

#[derive(Clone)]
pub struct Truncate;

impl PluginCommand for Truncate {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars truncate"
    }

    fn description(&self) -> &str {
        "Divide the date/datetime range into buckets."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(
                Type::Custom("expression".into()),
                Type::Custom("expression".into()),
            )])
            .required(
                "every",
                SyntaxShape::OneOf(vec![SyntaxShape::Duration, SyntaxShape::String]),
                "Period length for every interval (can be duration or str)",
            )
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Truncate a series of dates by period length",
            example: r#"seq date -b 2025-01-01 --periods 4 --increment 6wk -o "%Y-%m-%d %H:%M:%S" | polars into-df | polars as-datetime "%F %H:%M:%S" --naive | polars select datetime (polars col datetime | polars truncate 5d37m | polars as truncated)"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "datetime".to_string(),
                            vec![
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2025-01-01 00:00:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2025-02-12 00:00:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2025-03-26 00:00:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2025-05-07 00:00:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                            ],
                        ),
                        Column::new(
                            "truncated".to_string(),
                            vec![
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2024-12-30 16:49:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2025-02-08 21:45:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2025-03-21 02:41:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                                Value::date(
                                    DateTime::parse_from_str(
                                        "2025-05-05 08:14:00 +0000",
                                        "%Y-%m-%d %H:%M:%S %z",
                                    )
                                    .expect("date calculation should not fail in test"),
                                    Span::test_data(),
                                ),
                            ],
                        ),
                    ],
                    Some(NuSchema::new(Arc::new(Schema::from_iter(vec![
                        Field::new(
                            "datetime".into(),
                            DataType::Datetime(TimeUnit::Nanoseconds, None),
                        ),
                        Field::new(
                            "truncated".into(),
                            DataType::Datetime(TimeUnit::Nanoseconds, None),
                        ),
                    ])))),
                )
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        }]
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

    fn extra_description(&self) -> &str {
        r#"Each date/datetime is mapped to the start of its bucket using the corresponding local datetime. Note that weekly buckets start on Monday. Ambiguous results are localised using the DST offset of the original timestamp - for example, truncating '2022-11-06 01:30:00 CST' by '1h' results in '2022-11-06 01:00:00 CST', whereas truncating '2022-11-06 01:30:00 CDT' by '1h' results in '2022-11-06 01:00:00 CDT'.

        See Notes in documentation for full list of compatible string values for `every`: https://docs.pola.rs/api/python/stable/reference/expressions/api/polars.Expr.dt.truncate.html"#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![]
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let value = input.into_value(call.head)?;

    let every = match call.req(0)? {
        // handle Value::Duration input for maximum compatibility
        // duration types are always stored as nanoseconds
        Value::Duration { val, .. } => Ok(format!("{val}ns")),
        Value::String { val, .. } => Ok(val.clone()),
        x => Err(ShellError::IncompatibleParametersSingle {
            msg: format!("Expected duration or str type but got {}", x.get_type()),
            span: value.span(),
        }),
    }?;

    match PolarsPluginObject::try_from_value(plugin, &value)? {
        PolarsPluginObject::NuExpression(expr) => {
            let res: NuExpression = expr
                .into_polars()
                .dt()
                .truncate(Expr::Literal(LiteralValue::Dyn(DynLiteralValue::Str(
                    PlSmallStr::from_string(every),
                ))))
                .into();
            res.to_pipeline_data(plugin, engine, call.head)
        }
        _ => Err(cant_convert_err(&value, &[PolarsPluginType::NuExpression])),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command_with_decls;
    use nu_command::SeqDate;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command_with_decls(&Truncate, vec![Box::new(SeqDate)])
    }
}
