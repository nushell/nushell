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
pub struct SelectorEndsWith;

impl PluginCommand for SelectorEndsWith {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector ends-with"
    }

    fn description(&self) -> &str {
        "Select columns that end with the given substring(s)."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "prefix",
                SyntaxShape::String,
                "Select columns that end with the given substring(s).",
            )
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: r#"{
        "foo": ["x", "y"],
        "bar": [123, 456],
        "baz": [2.0, 5.5],
        "zap": [false, true],
    } |
    polars into-df --as-columns |
    polars select (polars selector ends-with z) |
    polars sort-by baz |
    polars collect"#,
                description: "Match columns ending with a 'z'",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "baz" => [2.0, 5.5],
                        )
                        .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: r#"{
        "foo": ["x", "y"],
        "bar": [123, 456],
        "baz": [2.0, 5.5],
        "zap": [false, true],
    } |
    polars into-df --as-columns |
    polars select (polars selector ends-with z r) |
    polars sort-by bar baz |
    polars collect "#,
                description: "Match columns ending with *either* the letter 'z' or 'r'",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "bar" => [123, 456],
                            "baz" => [2.0, 5.5],
                        )
                        .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: r#"{
        "foo": ["x", "y"],
        "bar": [123, 456],
        "baz": [2.0, 5.5],
        "zap": [false, true],
    } |
    polars into-df --as-columns |
    polars select (polars selector ends-with z | polars selector not) |
    polars sort-by foo bar zap |
    polars collect"#,
                description: "Match columns ending with *except* the letter 'z'",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "foo" => ["x", "y"],
                            "bar" => [123, 456],
                            "zap" => [false, true],
                        )
                        .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["columns", "select", "ends-with"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let suffixes: Result<Vec<String>, ShellError> =
            call.positional.iter().try_fold(Vec::new(), |mut acc, arg| {
                let s = arg.as_str()?;
                let suffix = fancy_regex::escape(s).to_string();
                acc.push(suffix);
                Ok(acc)
            });

        let suffixes_joined = suffixes?.join("|");
        let selector = Selector::Matches(format!("({suffixes_joined})$").into());
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
        test_polars_plugin_command(&SelectorEndsWith)
    }
}
