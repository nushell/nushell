use crate::{
    PolarsPlugin,
    dataframe::values::NuSelector,
    values::{CustomValueSupport, PolarsPluginType},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, SyntaxShape, Type};
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
                "Names of columns to select",
            )
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Create a selector for columns by name",
            example: "polars selector by-name foo bar",
            result: None,
        }]
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
