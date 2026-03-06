use crate::values::NuDataFrame;
use crate::{
    PolarsPlugin,
    dataframe::values::NuSelector,
    values::{CustomValueSupport, PolarsPluginType},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, Signature, Span, SyntaxShape, Type,
};
use polars::df;
use polars::prelude::Selector;

#[derive(Clone)]
pub struct SelectorMatches;

impl PluginCommand for SelectorMatches {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector matches"
    }

    fn description(&self) -> &str {
        "Select all columns that match the given regex pattern."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "pattern",
                SyntaxShape::String,
                "A valid regular expression pattern, compatible with the rust `regex crate <https://docs.rs/regex/latest/regex/>",
            )
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: r#"
                {
                    "foo": ["x", "y"],
                    "bar": [123, 456],
                    "baz": [2.0, 5.5],
                    "zap": [0, 1],
                } |
                polars into-df --as-columns |
                polars select (polars selector matches "[^z]a") |
                polars sort-by bar baz |
                polars collect
                "#,
                description: "Match column names containing an 'a', preceded by a character that is not 'z'",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "bar" => [123, 456],
                            "baz" => [2.0, 5.5],
                        )
                        .expect("Failed to create expected DataFrame"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: r#"
                {
                    "foo": ["x", "y"],
                    "bar": [123, 456],
                    "baz": [2.0, 5.5],
                    "zap": [0, 1],
                } |
                polars into-df --as-columns |
                polars select (polars selector matches "(?i)R|z$" | polars selector not) |
                polars sort-by foo zap |
                polars collect
                "#,
                description: "Do not match column names ending in 'R' or 'z' (case-insensitively)",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "foo" => ["x", "y"],
                            "zap" => [0, 1],
                        )
                        .expect("Failed to create expected DataFrame"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["columns", "select", "matches"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let n: String = call.req(0)?;

        let selector = Selector::Matches(n.into());
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
        test_polars_plugin_command(&SelectorMatches)
    }
}
