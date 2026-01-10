use crate::{
    PolarsPlugin,
    dataframe::values::NuSelector,
    values::{CustomValueSupport, PolarsPluginType},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, Type};
use polars::prelude::Selector;

#[derive(Clone)]
pub struct SelectorAll;

impl PluginCommand for SelectorAll {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector all"
    }

    fn description(&self) -> &str {
        "Creates a selector that selects all columns."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        use crate::values::{Column, NuDataFrame};
        use nu_protocol::{Span, Value};

        vec![
            Example {
                description: "Create a selector for all columns",
                example: "polars selector all",
                result: None,
            },
            Example {
                description: "Multiply all columns by 2 using with-column",
                example: r#"[[a b]; [1 2] [3 4]]
                    | polars into-df
                    | polars with-column ((polars selector all) * 2)
                    | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(2), Value::test_int(6)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(4), Value::test_int(8)],
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
        vec!["wildcard", "columns", "select"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();

        let selector = Selector::Wildcard;
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
        test_polars_plugin_command(&SelectorAll)
    }
}
