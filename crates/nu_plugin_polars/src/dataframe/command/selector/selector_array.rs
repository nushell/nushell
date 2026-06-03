use crate::{
    PolarsPlugin,
    dataframe::values::NuSelector,
    values::{CustomValueSupport, PolarsPluginType},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, SyntaxShape, Type};
use polars::prelude::{DataTypeSelector, Selector};

#[derive(Clone)]
pub struct SelectorArray;

impl PluginCommand for SelectorArray {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector array"
    }

    fn description(&self) -> &str {
        "Select all array columns. Optionally filter by fixed width."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "width",
                SyntaxShape::Int,
                "Only select arrays with this fixed width.",
                None,
            )
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "polars selector array",
                description: "Create a selector for all array columns",
                result: None,
            },
            Example {
                example: "polars selector array --width 3",
                description: "Create a selector for fixed-width arrays of size 3",
                result: None,
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["columns", "select", "array", "fixed"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        mut input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.take_metadata();
        let width: Option<i64> = call.get_flag("width")?;
        let width_usize = width.map(|w| w as usize);

        let selector = Selector::ByDType(DataTypeSelector::Array(None, width_usize));
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
        test_polars_plugin_command(&SelectorArray)
    }
}
