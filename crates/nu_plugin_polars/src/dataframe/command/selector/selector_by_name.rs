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
pub struct SelectorByName;

impl PluginCommand for SelectorByName {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector by-name"
    }

    fn description(&self) -> &str {
        "Creates a selector that selects columns by name."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "column names",
                SyntaxShape::String,
                "Names of columns to select.",
            )
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Create a selector for columns by name",
                example: "polars selector by-name foo bar",
                result: None,
            },
            Example {
                description: "Add 10 to specific columns using with-column",
                example: r#"[[a b c]; [1 2 3] [4 5 6]]
                    | polars into-df
                    | polars with-column ((polars selector by-name a c) + 10)
                    | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(11), Value::test_int(14)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(2), Value::test_int(5)],
                            ),
                            Column::new(
                                "c".to_string(),
                                vec![Value::test_int(13), Value::test_int(16)],
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
        vec!["columns", "select", "name"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let names: Vec<String> = call.rest(0)?;

        if names.is_empty() {
            return Err(LabeledError::new("Missing column names")
                .with_label("At least one column name is required", call.head));
        }

        let selector = Selector::ByName {
            names: names.into_iter().map(|s| s.into()).collect(),
            strict: true,
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
        test_polars_plugin_command(&SelectorByName)
    }
}
