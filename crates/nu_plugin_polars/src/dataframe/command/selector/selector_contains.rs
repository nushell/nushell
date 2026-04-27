use crate::values::NuDataFrame;
use crate::{
    PolarsPlugin,
    dataframe::values::NuSelector,
    values::{CustomValueSupport, PolarsPluginType},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
};
use polars::df;
use polars::prelude::Selector;

#[derive(Clone)]
pub struct SelectorContains;

impl PluginCommand for SelectorContains {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector contains"
    }

    fn description(&self) -> &str {
        "Select columns whose names contain the given literal substring(s)."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "substring",
                SyntaxShape::String,
                "Literal substring(s) to search for in column names.",
            )
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: r#"{
    "foo_bar": [1.0, 2.0],
    "foo_baz": [3.0, 4.0],
    "qux": [5, 6],
} |
polars into-df --as-columns |
polars select (polars selector contains foo) |
polars sort-by foo_bar foo_baz |
polars collect"#,
                description: "Select columns whose names contain 'foo'",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "foo_bar" => [1.0, 2.0],
                            "foo_baz" => [3.0, 4.0],
                        )
                        .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: r#"{
    "foo_x": [1, 2],
    "bar_x": [3, 4],
    "baz": [5, 6],
} |
polars into-df --as-columns |
polars select (polars selector contains foo bar) |
polars sort-by foo_x bar_x |
polars collect"#,
                description: "Select columns whose names contain 'foo' or 'bar'",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "foo_x" => [1i64, 2i64],
                            "bar_x" => [3i64, 4i64],
                        )
                        .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["columns", "select", "contains", "substring"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        mut input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.take_metadata();
        let substrings: Result<Vec<String>, ShellError> =
            call.positional.iter().try_fold(Vec::new(), |mut acc, arg| {
                let s = arg.as_str()?;
                acc.push(fancy_regex::escape(s).to_string());
                Ok(acc)
            });

        let pattern = substrings?.join("|");
        let selector = Selector::Matches(pattern.into());
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
        test_polars_plugin_command(&SelectorContains)
    }
}
