use crate::dataframe::values::NuExpression;
use crate::values::{
    cant_convert_err, CustomValueSupport, NuDataFrame, PolarsPluginObject, PolarsPluginType,
};
use crate::PolarsPlugin;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type,
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
            .input_output_type(
                Type::Custom("expression".into()),
                Type::Custom("expression".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Var aggregation for a group-by",
            example: r#"[[a b]; [one 2] [one 2] [two 1] [two 1]]
    | polars into-df
    | polars group-by a
    | polars agg (polars col b | polars var)
    | polars collect"#,
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
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let value = input.into_value(call.head)?;
        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuExpression(expr) => command_expr(plugin, engine, call, expr),
            _ => Err(cant_convert_err(&value, &[PolarsPluginType::NuExpression])),
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
    let res: NuExpression = expr.into_polars().var(1).into();
    res.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;
    use nu_protocol::ShellError;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ExprVar)
    }
}
