use crate::{
    dataframe::values::{Column, NuDataFrame, NuExpression},
    values::{cant_convert_err, CustomValueSupport, PolarsPluginObject, PolarsPluginType},
    PolarsPlugin,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::prelude::{is_in, lit, DataType, IntoSeries};

#[derive(Clone)]
pub struct ExprIsIn;

impl PluginCommand for ExprIsIn {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars is-in"
    }

    fn usage(&self) -> &str {
        "Creates an is-in expression or checks to see if the elements are contained in the right series"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("list", SyntaxShape::Any, "List to check if values are in")
            .input_output_types(vec![
                (
                    Type::Custom("expression".into()),
                    Type::Custom("expression".into()),
                ),
                (
                    Type::Custom("dataframe".into()),
                    Type::Custom("dataframe".into()),
                ),
            ])
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Creates a is-in expression",
                example: r#"let df = ([[a b]; [one 1] [two 2] [three 3]] | polars into-df);
            $df | polars with-column (polars col a | polars is-in [one two] | polars as a_in)"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![
                                    Value::test_string("one"),
                                    Value::test_string("two"),
                                    Value::test_string("three"),
                                ],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                            ),
                            Column::new(
                                "a_in".to_string(),
                                vec![
                                    Value::test_bool(true),
                                    Value::test_bool(true),
                                    Value::test_bool(false),
                                ],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Checks if elements from a series are contained in right series",
                example: r#"let other = ([1 3 6] | polars into-df);
            [5 6 6 6 8 8 8] | polars into-df | polars is-in $other"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "is_in".to_string(),
                            vec![
                                Value::test_bool(false),
                                Value::test_bool(true),
                                Value::test_bool(true),
                                Value::test_bool(true),
                                Value::test_bool(false),
                                Value::test_bool(false),
                                Value::test_bool(false),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["check", "contained", "is-contain", "match"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let value = input.into_value(call.head);

        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuDataFrame(df) => command_df(plugin, engine, call, df),
            PolarsPluginObject::NuLazyFrame(lazy) => {
                command_df(plugin, engine, call, lazy.collect(call.head)?)
            }
            PolarsPluginObject::NuExpression(expr) => command_expr(plugin, engine, call, expr),
            _ => Err(cant_convert_err(
                &value,
                &[
                    PolarsPluginType::NuDataFrame,
                    PolarsPluginType::NuLazyFrame,
                    PolarsPluginType::NuExpression,
                ],
            )),
        }
        .map_err(LabeledError::from)
    }
}

fn command_expr(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    expr: NuExpression,
) -> Result<PipelineData, ShellError> {
    let list: Vec<Value> = call.req(0)?;

    let values = NuDataFrame::try_from_columns(vec![Column::new("list".to_string(), list)], None)?;
    let list = values.as_series(call.head)?;

    if matches!(list.dtype(), DataType::Object(..)) {
        return Err(ShellError::IncompatibleParametersSingle {
            msg: "Cannot use a mixed list as argument".into(),
            span: call.head,
        });
    }

    let expr: NuExpression = expr.to_polars().is_in(lit(list)).into();
    expr.to_pipeline_data(plugin, engine, call.head)
}

fn command_df(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let other_value: Value = call.req(0)?;
    let other_span = other_value.span();
    let other_df = NuDataFrame::try_from_value_coerce(plugin, &other_value, call.head)?;
    let other = other_df.as_series(other_span)?;
    let series = df.as_series(call.head)?;

    let mut res = is_in(&series, &other)
        .map_err(|e| ShellError::GenericError {
            error: "Error finding in other".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?
        .into_series();

    res.rename("is_in");

    let mut new_df = NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)?;
    new_df.from_lazy = df.from_lazy;
    new_df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ExprIsIn)
    }
}
