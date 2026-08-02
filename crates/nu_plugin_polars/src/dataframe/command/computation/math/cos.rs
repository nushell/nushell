use crate::PolarsPlugin;
use crate::values::{
    CustomValueSupport, NuDataFrame, NuExpression, PolarsPluginObject, PolarsPluginType,
    cant_convert_err,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, ShellError, Signature, Span};
use polars::df;

#[derive(Clone)]
pub struct ExprMathCos;

impl PluginCommand for ExprMathCos {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars math cos"
    }

    fn description(&self) -> &str {
        "Compute the element-wise cosine of a column expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
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
            description: "Compute the element-wise cosine of a float column",
            example: "[[a]; [0.0] [3.141592653589793]] | 
    polars into-df | 
    polars select (polars col a | polars math cos) | 
    polars collect",
            result: Some(
                NuDataFrame::from(
                    df!("a" => [1.0f64, -1.0f64]).expect("simple df for test should not fail"),
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
    NuExpression::from(expr.into_polars().cos()).to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ExprMathCos)
    }
}
