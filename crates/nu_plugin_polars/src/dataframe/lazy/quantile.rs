use crate::{
    dataframe::values::{Column, NuDataFrame, NuLazyFrame},
    values::{
        cant_convert_err, to_pipeline_data, CustomValueSupport, NuExpression, PolarsPluginObject,
        PolarsPluginType,
    },
    PolarsPlugin,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::prelude::{lit, QuantileInterpolOptions};

#[derive(Clone)]
pub struct LazyQuantile;

impl PluginCommand for LazyQuantile {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars quantile"
    }

    fn usage(&self) -> &str {
        "Aggregates the columns to the selected quantile."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "quantile",
                SyntaxShape::Number,
                "quantile value for quantile operation",
            )
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
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "quantile value from columns in a dataframe",
                example: "[[a b]; [6 2] [1 4] [4 1]] | polars into-df | polars quantile 0.5",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new("a".to_string(), vec![Value::test_float(4.0)]),
                            Column::new("b".to_string(), vec![Value::test_float(2.0)]),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Quantile aggregation for a group-by",
                example: r#"[[a b]; [one 2] [one 4] [two 1]]
    | polars into-df
    | polars group-by a
    | polars agg (polars col b | polars quantile 0.5)"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_string("one"), Value::test_string("two")],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_float(4.0), Value::test_float(1.0)],
                            ),
                        ],
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
        let value = input.into_value(call.head);
        let quantile: f64 = call.req(0)?;
        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuDataFrame(df) => {
                command(plugin, engine, call, df.lazy(), quantile)
            }
            PolarsPluginObject::NuLazyFrame(lazy) => command(plugin, engine, call, lazy, quantile),
            PolarsPluginObject::NuExpression(expr) => {
                let expr: NuExpression = expr
                    .to_polars()
                    .quantile(lit(quantile), QuantileInterpolOptions::default())
                    .into();
                to_pipeline_data(plugin, engine, call.head, expr)
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
        .map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
    quantile: f64,
) -> Result<PipelineData, ShellError> {
    let lazy = NuLazyFrame::new(
        lazy.from_eager,
        lazy.to_polars()
            .quantile(lit(quantile), QuantileInterpolOptions::default())
            .map_err(|e| ShellError::GenericError {
                error: "Dataframe Error".into(),
                msg: e.to_string(),
                help: None,
                span: None,
                inner: vec![],
            })?,
    );

    to_pipeline_data(plugin, engine, call.head, lazy)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&LazyQuantile)
    }
}
