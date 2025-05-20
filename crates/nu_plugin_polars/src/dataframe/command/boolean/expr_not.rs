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

pub struct ExprNot;

impl PluginCommand for ExprNot {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars expr-not"
    }

    fn description(&self) -> &str {
        "Creates a not expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(
                Type::Custom("expression".into()),
                Type::Custom("expression".into()),
            )])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Creates a not expression",
                example: "(polars col a) > 2) | polars expr-not",
                result: None,
            },
            Example {
                description: "Adds a column showing which values of col a are not greater than 2",
                example: "[[a]; [1] [2] [3] [4] [5]] | polars into-df 
                    | polars with-column [(((polars col a) > 2)
                    | polars expr-not
                    | polars as a_expr_not)]
                    | polars collect
                    | polars sort-by a",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "a" => [1, 2, 3, 4, 5],
                            "b" => [true, true, false, false, false]
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
    NuExpression::from(expr.into_polars().not()).to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;
    use nu_protocol::ShellError;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ExprNot)
    }
}
