use crate::{
    dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame},
    values::CustomValueSupport,
    PolarsPlugin,
};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct LazyFilter;

impl PluginCommand for LazyFilter {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars filter"
    }

    fn usage(&self) -> &str {
        "Filter dataframe based in expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "filter expression",
                SyntaxShape::Any,
                "Expression that define the column selection",
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Filter dataframe using an expression",
            example:
                "[[a b]; [6 2] [4 2] [2 2]] | polars into-df | polars filter ((polars col a) >= 4)",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_int(6), Value::test_int(4)],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_int(2), Value::test_int(2)],
                        ),
                    ],
                    None,
                )
                .expect("simple df for test should not fail")
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
        let expr_value: Value = call.req(0)?;
        let filter_expr = NuExpression::try_from_value(plugin, &expr_value)?;
        let pipeline_value = input.into_value(call.head);
        let lazy = NuLazyFrame::try_from_value_coerce(plugin, &pipeline_value)?;
        command(plugin, engine, call, lazy, filter_expr).map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
    filter_expr: NuExpression,
) -> Result<PipelineData, ShellError> {
    let lazy = NuLazyFrame::new(
        lazy.from_eager,
        lazy.to_polars().filter(filter_expr.to_polars()),
    );
    lazy.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&LazyFilter)
    }
}
