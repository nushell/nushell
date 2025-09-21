use crate::{
    PolarsPlugin,
    dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame},
    values::{CustomValueSupport, PolarsPluginObject, PolarsPluginType, cant_convert_err},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct LazyFillNull;

impl PluginCommand for LazyFillNull {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars fill-null"
    }

    fn description(&self) -> &str {
        "Replaces NULL values with the given expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "fill",
                SyntaxShape::Any,
                "Expression to use to fill the null values",
            )
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
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Fills the null values by 0",
                example: "[1 2 2 3 3] | polars into-df | polars shift 2 | polars fill-null 0",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "0".to_string(),
                            vec![
                                Value::test_int(0),
                                Value::test_int(0),
                                Value::test_int(1),
                                Value::test_int(2),
                                Value::test_int(2),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Fills the null values in expression",
                example: "[[a]; [1] [2] [2] [3] [3]]
                    | polars into-df
                    | polars select (polars col a | polars shift 2 | polars fill-null 0)
                    | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "a".to_string(),
                            vec![
                                Value::test_int(0),
                                Value::test_int(0),
                                Value::test_int(1),
                                Value::test_int(2),
                                Value::test_int(2),
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

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let fill: Value = call.req(0)?;
        let value = input.into_value(call.head)?;

        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuDataFrame(df) => cmd_lazy(plugin, engine, call, df.lazy(), fill),
            PolarsPluginObject::NuLazyFrame(lazy) => cmd_lazy(plugin, engine, call, lazy, fill),
            PolarsPluginObject::NuExpression(expr) => cmd_expr(plugin, engine, call, expr, fill),
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
        .map(|pd| pd.set_metadata(metadata))
    }
}

fn cmd_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
    fill: Value,
) -> Result<PipelineData, ShellError> {
    let expr = NuExpression::try_from_value(plugin, &fill)?.into_polars();
    let lazy = NuLazyFrame::new(lazy.from_eager, lazy.to_polars().fill_null(expr));
    lazy.to_pipeline_data(plugin, engine, call.head)
}

fn cmd_expr(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    expr: NuExpression,
    fill: Value,
) -> Result<PipelineData, ShellError> {
    let fill = NuExpression::try_from_value(plugin, &fill)?.into_polars();
    let expr: NuExpression = expr.into_polars().fill_null(fill).into();
    expr.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&LazyFillNull)
    }
}
