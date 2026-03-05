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
pub struct SelectorNumeric;

impl PluginCommand for SelectorNumeric {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector numeric"
    }

    fn description(&self) -> &str {
        "Select all numeric columns."
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
        "zap": [0, 0],
    } |
    polars into-df --as-columns |
    polars select (polars selector numeric) |
    polars sort-by bar baz zap |
    polars collect"#,
            description: "Select all numeric columns",
            result: Some(
                NuDataFrame::from(
                    df!(
                        "bar" => [123, 456],
                        "baz" => [2.0, 5.5],
                        "zap" => [0, 0],
                    )
                    .expect("simple df for test should not fail"),
                )
                .into_value(Span::test_data()),
            ),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["columns", "select", "numeric"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();

        let selector = Selector::ByDType(DataTypeSelector::Numeric);
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
        test_polars_plugin_command(&SelectorNumeric)
    }
}
