use crate::PolarsPlugin;
use crate::dataframe::values::NuExpression;
use crate::values::{
    CustomValueSupport, NuDataFrame, PolarsPluginObject, PolarsPluginType, cant_convert_err,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type,
};
use polars::df;
use polars::series::Series;

pub struct ExprImplode;

impl PluginCommand for ExprImplode {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars implode"
    }

    fn description(&self) -> &str {
        "Aggregates values into a list."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(
                Type::Custom("expression".into()),
                Type::Custom("expression".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Create two lists for columns a and b with all the rows as values.",
            example: "[[a b]; [1 4] [2 5] [3 6]] | polars into-df | polars select (polars col '*' | polars implode) | polars collect",
            result: Some(
                NuDataFrame::from(
                    df!(
                        "a"=> [[1i64, 2, 3].iter().collect::<Series>()],
                        "b"=> [[4i64, 5, 6].iter().collect::<Series>()],
                    )
                    .expect("should not fail"),
                )
                .into_value(Span::unknown()),
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
        let value = input.into_value(call.head)?;
        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuExpression(expr) => command_expr(plugin, engine, call, expr),
            _ => Err(cant_convert_err(&value, &[PolarsPluginType::NuExpression])),
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
    let res: NuExpression = expr.into_polars().implode().into();
    res.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;
    use nu_protocol::ShellError;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ExprImplode)
    }
}
