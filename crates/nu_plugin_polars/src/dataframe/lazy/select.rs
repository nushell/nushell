use crate::{
    dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame},
    values::{to_pipeline_data, CustomValueSupport},
    PolarsPlugin,
};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, Signature, Span, SyntaxShape, Type, Value,
};
#[derive(Clone)]
pub struct LazySelect;

impl PluginCommand for LazySelect {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars select"
    }

    fn usage(&self) -> &str {
        "Selects columns from lazyframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "select expressions",
                SyntaxShape::Any,
                "Expression(s) that define the column selection",
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Select a column from the dataframe",
            example: "[[a b]; [6 2] [4 2] [2 2]] | polars into-df | polars select a",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "a".to_string(),
                        vec![Value::test_int(6), Value::test_int(4), Value::test_int(2)],
                    )],
                    None,
                )
                .expect("simple df for test should not fail")
                .base_value(Span::test_data())
                .expect("rendering base value should not fail"),
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
        let vals: Vec<Value> = call.rest(0)?;
        let expr_value = Value::list(vals, call.head);
        let expressions = NuExpression::extract_exprs(plugin, expr_value)?;

        let pipeline_value = input.into_value(call.head);
        let lazy = NuLazyFrame::try_from_value_coerce(plugin, &pipeline_value)?;
        let lazy = NuLazyFrame::new(lazy.from_eager, lazy.to_polars().select(&expressions));
        to_pipeline_data(plugin, engine, call.head, lazy).map_err(LabeledError::from)
    }
}

// todo: fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![Box::new(LazySelect {})])
//     }
// }
