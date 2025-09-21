use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, Signature, Span, SyntaxShape, Type, Value,
};

use crate::{
    PolarsPlugin,
    dataframe::values::{Column, NuDataFrame},
    values::CustomValueSupport,
};

use super::explode::explode;

#[derive(Clone)]
pub struct LazyFlatten;

impl PluginCommand for LazyFlatten {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars flatten"
    }

    fn description(&self) -> &str {
        "An alias for polars explode."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "columns",
                SyntaxShape::String,
                "columns to flatten, only applicable for dataframes",
            )
            .input_output_types(vec![
                (
                    Type::Custom("expression".into()),
                    Type::Custom("expression".into()),
                ),
                (
                    Type::Custom("dataframe".into()),
                    Type::Custom("dataframe".into()),
                ),
            ])
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Flatten the specified dataframe",
                example: "[[id name hobbies]; [1 Mercy [Cycling Knitting]] [2 Bob [Skiing Football]]] | polars into-df | polars flatten hobbies | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "id".to_string(),
                                vec![
                                    Value::test_int(1),
                                    Value::test_int(1),
                                    Value::test_int(2),
                                    Value::test_int(2),
                                ],
                            ),
                            Column::new(
                                "name".to_string(),
                                vec![
                                    Value::test_string("Mercy"),
                                    Value::test_string("Mercy"),
                                    Value::test_string("Bob"),
                                    Value::test_string("Bob"),
                                ],
                            ),
                            Column::new(
                                "hobbies".to_string(),
                                vec![
                                    Value::test_string("Cycling"),
                                    Value::test_string("Knitting"),
                                    Value::test_string("Skiing"),
                                    Value::test_string("Football"),
                                ],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Select a column and flatten the values",
                example: "[[id name hobbies]; [1 Mercy [Cycling Knitting]] [2 Bob [Skiing Football]]] | polars into-df | polars select (polars col hobbies | polars flatten)",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "hobbies".to_string(),
                            vec![
                                Value::test_string("Cycling"),
                                Value::test_string("Knitting"),
                                Value::test_string("Skiing"),
                                Value::test_string("Football"),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
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
        explode(plugin, engine, call, input)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), nu_protocol::ShellError> {
        test_polars_plugin_command(&LazyFlatten)
    }
}
