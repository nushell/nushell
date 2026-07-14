use crate::PolarsPlugin;
use crate::values::{
    CustomValueSupport, NuExpression, PolarsPluginObject, PolarsPluginType, cant_convert_err,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, ShellError, Signature};

#[derive(Clone)]
pub struct ExprMathLog1p;

impl PluginCommand for ExprMathLog1p {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars math log1p"
    }

    fn description(&self) -> &str {
        "Compute the element-wise natural log of 1 + x for a column expression."
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
            description: "Compute the element-wise natural log of 1 + x for a column",
            example: "[[a]; [0] [1] [2]] | 
    polars into-df | 
    polars select (polars col a | polars math log1p) | 
    polars collect",
            result: None,
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
    NuExpression::from(expr.into_polars().log1p()).to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ExprMathLog1p)
    }
}
