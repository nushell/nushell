use crate::PolarsPlugin;
use crate::values::{
    CustomValueSupport, NuDataFrame, NuExpression, PolarsPluginObject, PolarsPluginType,
    cant_convert_err,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
    shell_error::generic::GenericError,
};
use polars::df;
use polars::prelude::Expr;

#[derive(Clone)]
pub struct ExprMathDot;

impl PluginCommand for ExprMathDot {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars math dot"
    }

    fn description(&self) -> &str {
        "Compute the dot product of two column expressions."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "other",
                SyntaxShape::Any,
                "Expression to compute dot product with.",
            )
            .input_output_types(vec![
                (
                    PolarsPluginType::NuExpression.into(),
                    PolarsPluginType::NuExpression.into(),
                ),
                (
                    PolarsPluginType::NuSelector.into(),
                    PolarsPluginType::NuExpression.into(),
                ),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Compute the dot product of two integer columns",
            example: "[[a b]; [0 0] [1 1] [2 2] [3 3] [4 4] [5 5]] | 
    polars into-df | 
    polars select (polars col a | polars math dot (polars col b) | polars as ab) | 
    polars collect",
            result: Some(
                NuDataFrame::from(
                    df!("ab" => [55.0f64]).expect("simple df for test should not fail"),
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
        mut input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.take_metadata();
        let value = input.into_value(call.head)?;
        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuExpression(expr) => command_expr(plugin, engine, call, expr),
            PolarsPluginObject::NuSelector(selector) => {
                let expr = selector.into_expr();
                command_expr(plugin, engine, call, expr)
            }
            _ => Err(cant_convert_err(
                &value,
                &[PolarsPluginType::NuExpression, PolarsPluginType::NuSelector],
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
    let other_value: Value = call.req(0)?;
    let other: Expr = match PolarsPluginObject::try_from_value(plugin, &other_value)? {
        PolarsPluginObject::NuExpression(e) => e.into_polars(),
        _ => {
            return Err(ShellError::Generic(GenericError::new(
                "Second expression to compute dot product with must be provided",
                "",
                call.head,
            )));
        }
    };
    NuExpression::from(expr.into_polars().dot(other)).to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ExprMathDot)
    }
}
