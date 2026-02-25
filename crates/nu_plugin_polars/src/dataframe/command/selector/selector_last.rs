use crate::values::{Column, NuDataFrame};
use crate::{
    PolarsPlugin,
    dataframe::values::NuSelector,
    values::{CustomValueSupport, PolarsPluginType},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, Signature, Span, SyntaxShape, Type, Value,
};
use polars::prelude::Selector;

#[derive(Clone)]
pub struct SelectorLast;

impl PluginCommand for SelectorLast {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector last"
    }

    fn description(&self) -> &str {
        "Creates a selector that selects the last column(s) by index."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional(
                "n",
                SyntaxShape::Int,
                "Number of columns to select from the end (default: 1).",
            )
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Create a selector for the last column",
                example: "polars selector last",
                result: None,
            },
            Example {
                description: "Create a selector for the last 3 columns",
                example: "polars selector last 3",
                result: None,
            },
            Example {
                description: "Create a new column from the last column using with-column",
                example: r#"[[a b c]; [1 2 3] [4 5 6]]
                    | polars into-df
                    | polars with-column ((polars selector last) * 100 | polars as c_times_100)
                    | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(1), Value::test_int(4)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(2), Value::test_int(5)],
                            ),
                            Column::new(
                                "c".to_string(),
                                vec![Value::test_int(3), Value::test_int(6)],
                            ),
                            Column::new(
                                "c_times_100".to_string(),
                                vec![Value::test_int(300), Value::test_int(600)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["columns", "select", "last", "end"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let n: Option<i64> = call.opt(0)?;

        let selector = match n {
            Some(count) if count > 0 => {
                let indices: Vec<i64> = ((-count)..0).collect();
                Selector::ByIndex {
                    indices: indices.into(),
                    strict: false,
                }
            }
            _ => Selector::ByIndex {
                indices: vec![-1].into(),
                strict: false,
            },
        };

        let nu_selector = NuSelector::from(selector);

        nu_selector
            .to_pipeline_data(plugin, engine, call.head)
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
        test_polars_plugin_command(&SelectorLast)
    }
}
