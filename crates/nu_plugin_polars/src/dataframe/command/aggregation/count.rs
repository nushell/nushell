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

pub struct ExprCount;

impl PluginCommand for ExprCount {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars count"
    }

    fn description(&self) -> &str {
        "Returns the number of non-null values in the column."
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
        // to add an example with a result that contains a null we will need to be able to
        // allow null values to be entered into the dataframe from nushell
        // and retain the correct dtype. Right now null values cause the dtype to be object
        vec![Example {
            description: "Count the number of non-null values in a column",
            example: r#"[[a]; ["foo"] ["bar"]] | polars into-df 
                    | polars select (polars col a | polars count) 
                    | polars collect"#,
            result: Some(
                NuDataFrame::from(
                    df!(
                        "a" => [2]
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
    NuExpression::from(expr.into_polars().count()).to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;
    use nu_protocol::ShellError;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ExprCount)
    }
}
