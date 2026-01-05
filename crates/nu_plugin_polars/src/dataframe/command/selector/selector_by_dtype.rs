use crate::{
    PolarsPlugin,
    dataframe::values::NuSelector,
    values::{CustomValueSupport, PolarsPluginType, str_to_dtype},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, SyntaxShape, Type,
};
use polars::prelude::{DataType, DataTypeSelector, Selector};

#[derive(Clone)]
pub struct SelectorByDtype;

impl PluginCommand for SelectorByDtype {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector by-dtype"
    }

    fn description(&self) -> &str {
        "Creates a selector that selects columns by data type."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "data types",
                SyntaxShape::String,
                "Data types to select (e.g., i64, f64, str, bool)",
            )
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Create a selector for numeric columns",
            example: "polars selector by-dtype i64 f64",
            result: None,
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["columns", "select", "type", "dtype"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let dtype_strs: Vec<String> = call.rest(0)?;

        if dtype_strs.is_empty() {
            return Err(LabeledError::new("Missing data types")
                .with_label("At least one data type is required", call.head));
        }

        let dtypes = dtype_strs
            .iter()
            .map(|s| str_to_dtype(s, call.head))
            .collect::<Result<Vec<DataType>, ShellError>>()
            .map_err(LabeledError::from)?;

        let selector = Selector::ByDType(DataTypeSelector::AnyOf(dtypes.into()));
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
        test_polars_plugin_command(&SelectorByDtype)
    }
}
