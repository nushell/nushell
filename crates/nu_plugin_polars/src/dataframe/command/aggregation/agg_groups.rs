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

pub struct ExprAggGroups;

impl PluginCommand for ExprAggGroups {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars agg-groups"
    }

    fn description(&self) -> &str {
        "Creates an agg_groups expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(
                Type::Custom("expression".into()),
                Type::Custom("expression".into()),
            )])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Get the group index of the group by operations.",
            example: r#"[[group value]; [one 94] [one 95] [one 96] [two 97] [two 98] [two 99]] 
                | polars into-df 
                | polars group-by group
                | polars agg (polars col value | polars agg-groups)
                | polars collect
                | polars sort-by group"#,
            result: Some(
                NuDataFrame::from(
                    df!(
                        "group"=> ["one", "two"],
                        "values" => [[0i64, 1, 2].iter().collect::<Series>(), [3i64, 4, 5].iter().collect::<Series>()],
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
    NuExpression::from(expr.into_polars().agg_groups()).to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;
    use nu_protocol::ShellError;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ExprAggGroups)
    }
}
