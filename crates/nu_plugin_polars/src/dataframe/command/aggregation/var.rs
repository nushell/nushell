use crate::PolarsPlugin;
use crate::dataframe::values::NuExpression;
use crate::values::{
    Column, CustomValueSupport, NuDataFrame, NuLazyFrame, PolarsPluginObject, PolarsPluginType,
    cant_convert_err,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Value,
};
use polars::df;

pub struct ExprVar;

impl PluginCommand for ExprVar {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars var"
    }

    fn description(&self) -> &str {
        "Create a var expression for an aggregation."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (
                    PolarsPluginType::NuExpression.into(),
                    PolarsPluginType::NuExpression.into(),
                ),
                (
                    PolarsPluginType::NuDataFrame.into(),
                    PolarsPluginType::NuDataFrame.into(),
                ),
                (
                    PolarsPluginType::NuLazyFrame.into(),
                    PolarsPluginType::NuLazyFrame.into(),
                ),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Var value from columns in a dataframe or aggregates columns to their var value",
                example: "[[a b]; [6 2] [4 2] [2 2]] | polars into-df | polars var | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new("a".to_string(), vec![Value::test_float(4.0)]),
                            Column::new("b".to_string(), vec![Value::test_float(0.0)]),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Var aggregation for a group-by",
                example: r#"[[a b]; [one 2] [one 2] [two 1] [two 1]]
                    | polars into-df
                    | polars group-by a
                    | polars agg (polars col b | polars var)
                    | polars collect
                    | polars sort-by a"#,
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "a" => &["one", "two"],
                            "b" => &[0.0, 0.0],
                        )
                        .expect("should not fail"),
                    )
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
        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuDataFrame(df) => command_lazy(plugin, engine, call, df.lazy()),
            PolarsPluginObject::NuLazyFrame(lazy) => command_lazy(plugin, engine, call, lazy),
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
        .map(|pd| pd.set_metadata(metadata))
    }
}

fn command_expr(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    expr: NuExpression,
) -> Result<PipelineData, ShellError> {
    NuExpression::from(expr.into_polars().var(1)).to_pipeline_data(plugin, engine, call.head)
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let res: NuLazyFrame = lazy.to_polars().var(1).into();

    res.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use crate::test::test_polars_plugin_command;

    use super::*;
    use nu_protocol::ShellError;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ExprVar)
    }
}
