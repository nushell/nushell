use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::prelude::LazyFrame;

use crate::{
    dataframe::values::{NuExpression, NuLazyFrame},
    values::CustomValueSupport,
    PolarsPlugin,
};

use super::super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct FilterWith;

impl PluginCommand for FilterWith {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars filter-with"
    }

    fn usage(&self) -> &str {
        "Filters dataframe using a mask or expression as reference."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "mask or expression",
                SyntaxShape::Any,
                "boolean mask used to filter data",
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe or lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Filter dataframe using an expression",
            example:
                "[[a b]; [1 2] [3 4]] | polars into-df | polars filter-with ((polars col a) > 1)",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new("a".to_string(), vec![Value::test_int(3)]),
                        Column::new("b".to_string(), vec![Value::test_int(4)]),
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
        let value = input.into_value(call.head);
        let lazy = NuLazyFrame::try_from_value_coerce(plugin, &value)?;
        command_lazy(plugin, engine, call, lazy).map_err(LabeledError::from)
    }
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let expr: Value = call.req(0)?;
    let expr = NuExpression::try_from_value(plugin, &expr)?;
    let lazy = lazy.apply_with_expr(expr, LazyFrame::filter);
    lazy.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&FilterWith)
    }
}
