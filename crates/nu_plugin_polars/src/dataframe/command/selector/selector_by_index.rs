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
pub struct SelectorByIndex;

impl PluginCommand for SelectorByIndex {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector by-index"
    }

    fn description(&self) -> &str {
        "Select columns by their index position. Supports negative indices (e.g., -1 for the last column)."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "index",
                SyntaxShape::Int,
                "Column index positions to select (negative indices count from the end).",
            )
            .switch(
                "not-strict",
                "Allow out-of-range indices without error.",
                None,
            )
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: r#"[[a b c]; [1 2 3] [4 5 6]]
    | polars into-df
    | polars select (polars selector by-index 0 2)
    | polars collect"#,
                description: "Select first and third columns by index",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(1), Value::test_int(4)],
                            ),
                            Column::new(
                                "c".to_string(),
                                vec![Value::test_int(3), Value::test_int(6)],
                            ),
                        ],
                        None,
                        Span::test_data(),
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: r#"[[a b c]; [1 2 3] [4 5 6]]
    | polars into-df
    | polars select (polars selector by-index -1)
    | polars collect"#,
                description: "Select the last column using a negative index",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "c".to_string(),
                            vec![Value::test_int(3), Value::test_int(6)],
                        )],
                        None,
                        Span::test_data(),
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["columns", "select", "index", "position"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        mut input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.take_metadata();
        let indices: Vec<i64> = call.rest(0)?;
        let strict = !call.has_flag("not-strict")?;

        let selector = Selector::ByIndex {
            indices: indices.into(),
            strict,
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
        test_polars_plugin_command(&SelectorByIndex)
    }
}
