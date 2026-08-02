use crate::PolarsPlugin;
use crate::values::{
    CustomValueSupport, NuDataFrame, NuExpression, PolarsPluginObject, PolarsPluginType,
    cant_convert_err,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use polars::df;
use polars::prelude::Literal;

#[derive(Clone)]
pub struct ExprMathLog;

impl PluginCommand for ExprMathLog {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars math log"
    }

    fn description(&self) -> &str {
        "Compute the element-wise logarithm of a column expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional(
                "base",
                SyntaxShape::Number,
                "Logarithm base (default: e, the natural logarithm).",
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
            description: "Compute the base-2 logarithm of a column",
            example: "[[a]; [0] [1] [2] [4] [8] [16]] | 
    polars into-df | 
    polars select (polars col a | polars math log 2 | polars as a_base2) | 
    polars collect",
            result: Some(
                NuDataFrame::from(
                    df!("a_base2" => [f64::NEG_INFINITY, 0.0, 1.0, 2.0, 3.0, 4.0])
                        .expect("simple df for test should not fail"),
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
    let base = match call.opt::<Value>(0)? {
        None => std::f64::consts::E.lit(),
        Some(value) => NuExpression::try_from_value(plugin, &value)?.into_polars(),
    };
    NuExpression::from(expr.into_polars().log(base)).to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ExprMathLog)
    }
}
