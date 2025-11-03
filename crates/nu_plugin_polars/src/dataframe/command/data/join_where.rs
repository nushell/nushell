use crate::{
    PolarsPlugin,
    dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame},
    values::{CustomValueSupport, PolarsPluginType},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct LazyJoinWhere;

impl PluginCommand for LazyJoinWhere {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars join-where"
    }

    fn description(&self) -> &str {
        "Joins a lazy frame with other lazy frame based on conditions."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("other", SyntaxShape::Any, "LazyFrame to join with")
            .required("condition", SyntaxShape::Any, "Condition")
            .input_output_types(vec![
                (
                    PolarsPluginType::NuDataFrame.into(),
                    PolarsPluginType::NuDataFrame.into(),
                ),
                (
                    PolarsPluginType::NuLazyFrame.into(),
                    PolarsPluginType::NuLazyFrame.into(),
                ),
            ])
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Join two lazy dataframes with a condition",
            example: r#"let df_a = ([[name cash];[Alice 5] [Bob 10]] | polars into-lazy)
    let df_b = ([[item price];[A 3] [B 7] [C 12]] | polars into-lazy)
    $df_a | polars join-where $df_b ((polars col cash) > (polars col price)) | polars collect"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "name".to_string(),
                            vec![
                                Value::test_string("Bob"),
                                Value::test_string("Bob"),
                                Value::test_string("Alice"),
                            ],
                        ),
                        Column::new(
                            "cash".to_string(),
                            vec![Value::test_int(10), Value::test_int(10), Value::test_int(5)],
                        ),
                        Column::new(
                            "item".to_string(),
                            vec![
                                Value::test_string("B"),
                                Value::test_string("A"),
                                Value::test_string("A"),
                            ],
                        ),
                        Column::new(
                            "price".to_string(),
                            vec![Value::test_int(7), Value::test_int(3), Value::test_int(3)],
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
        let other: Value = call.req(0)?;
        let other = NuLazyFrame::try_from_value_coerce(plugin, &other)?;
        let other = other.to_polars();

        let condition: Value = call.req(1)?;
        let condition = NuExpression::try_from_value(plugin, &condition)?;
        let condition = condition.into_polars();

        let pipeline_value = input.into_value(call.head)?;
        let lazy = NuLazyFrame::try_from_value_coerce(plugin, &pipeline_value)?;
        let from_eager = lazy.from_eager;
        let lazy = lazy.to_polars();

        let lazy = lazy
            .join_builder()
            .with(other)
            .force_parallel(true)
            .join_where(vec![condition]);

        let lazy = NuLazyFrame::new(from_eager, lazy);
        lazy.to_pipeline_data(plugin, engine, call.head)
            .map_err(LabeledError::from)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), nu_protocol::ShellError> {
        test_polars_plugin_command(&LazyJoinWhere)
    }
}
