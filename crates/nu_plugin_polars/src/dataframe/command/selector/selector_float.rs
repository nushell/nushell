use crate::values::NuDataFrame;
use crate::{
    PolarsPlugin,
    dataframe::values::NuSelector,
    values::{CustomValueSupport, PolarsPluginType},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, Span, Type};
use polars::df;
use polars::prelude::{DataTypeSelector, Selector};

#[derive(Clone)]
pub struct SelectorFloat;

impl PluginCommand for SelectorFloat {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector float"
    }

    fn description(&self) -> &str {
        "Select all float columns."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            example: r#"{
        "foo": ["x", "y"],
        "bar": [123, 456],
        "baz": [2.0, 5.5],
        "qux": [3.1, 2.7],
    } |
    polars into-df --as-columns |
    polars select (polars selector float) |
    polars sort-by baz qux |
    polars collect"#,
            description: "Select all float columns",
            result: Some(
                NuDataFrame::from(
                    df!(
                        "baz" => [2.0, 5.5],
                        "qux" => [3.1, 2.7],
                    )
                    .expect("simple df for test should not fail"),
                )
                .into_value(Span::test_data()),
            ),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["columns", "select", "float", "floating-point"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();

        let selector = Selector::ByDType(DataTypeSelector::Float);
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
        test_polars_plugin_command(&SelectorFloat)
    }
}
