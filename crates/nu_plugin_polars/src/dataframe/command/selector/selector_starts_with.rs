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
pub struct SelectorStartsWith;

impl PluginCommand for SelectorStartsWith {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector starts-with"
    }

    fn description(&self) -> &str {
        "Select columns that start with the given substring(s)."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "prefix",
                SyntaxShape::String,
                "Select columns that start with the given substring(s).",
            )
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: r#"{
        "foo": [1.0, 2.0],
        "bar": [3.0, 4.0],
        "baz": [5, 6],
        "zap": [7, 8],
    } |
    polars into-df --as-columns |
    polars select (polars selector starts-with b) |
    polars sort-by bar baz |
    polars collect"#,
                description: "Match columns starting with a 'b'",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "bar" => [3.0, 4.0],
                            "baz" => [5, 6],
                        )
                        .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: r#"{
        "foo": [1.0, 2.0],
        "bar": [3.0, 4.0],
        "baz": [5, 6],
        "zap": [7, 8],
    } |
    polars into-df --as-columns |
    polars select (polars selector starts-with b z) |
    polars sort-by bar baz zap |
    polars collect"#,
                description: "Match columns starting with *either* the letter 'b' or 'z'",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "bar" => [3.0, 4.0],
                            "baz" => [5, 6],
                            "zap" => [7, 8],
                        )
                        .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["columns", "select", "starts-with"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let prefixes: Result<Vec<String>, ShellError> =
            call.positional.iter().try_fold(Vec::new(), |mut acc, arg| {
                let s = arg.as_str()?;
                let prefix = fancy_regex::escape(s).to_string();
                acc.push(prefix);
                Ok(acc)
            });

        let prefixes_joined = prefixes?.join("|");
        let selector = Selector::Matches(format!("^{prefixes_joined}").into());
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
        test_polars_plugin_command(&SelectorStartsWith)
    }
}
