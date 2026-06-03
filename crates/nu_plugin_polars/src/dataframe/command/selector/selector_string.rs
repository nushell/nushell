use crate::values::NuDataFrame;
use crate::{
    PolarsPlugin,
    dataframe::values::NuSelector,
    values::{CustomValueSupport, PolarsPluginType},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, Span, Type};
use polars::df;
use polars::prelude::{DataType, DataTypeSelector, Selector};
use std::sync::Arc;

#[derive(Clone)]
pub struct SelectorString;

impl PluginCommand for SelectorString {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector string"
    }

    fn description(&self) -> &str {
        "Select all string columns. Use `--include-categorical` to also select categorical columns."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch(
                "include-categorical",
                "Also include categorical columns in the selection.",
                Some('c'),
            )
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: r#"{
    "name": ["Alice", "Bob"],
    "age": [30, 25],
    "active": [true, false],
} |
polars into-df --as-columns |
polars select (polars selector string) |
polars collect"#,
                description: "Select all string columns",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "name" => ["Alice", "Bob"],
                        )
                        .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: "polars selector string --include-categorical",
                description: "Create a selector for string and categorical columns",
                result: None,
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["columns", "select", "string", "text", "str"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        mut input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.take_metadata();
        let include_categorical = call.has_flag("include-categorical")?;

        let selector = if include_categorical {
            Selector::ByDType(DataTypeSelector::Union(
                Arc::new(DataTypeSelector::AnyOf(vec![DataType::String].into())),
                Arc::new(DataTypeSelector::Categorical),
            ))
        } else {
            Selector::ByDType(DataTypeSelector::AnyOf(vec![DataType::String].into()))
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
        test_polars_plugin_command(&SelectorString)
    }
}
