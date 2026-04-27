use crate::values::NuDataFrame;
use crate::{
    PolarsPlugin,
    dataframe::values::NuSelector,
    values::{CustomValueSupport, PolarsPluginType},
};
use log::debug;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, Span, Type};
use polars::df;
use polars::prelude::Selector;

#[derive(Clone)]
pub struct SelectorDigit;

impl PluginCommand for SelectorDigit {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector digit"
    }

    fn description(&self) -> &str {
        r#"Select columns whose names consist entirely of digit characters. By default uses Unicode decimal digits; use `--ascii-only` to restrict to ASCII 0-9."#
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch(
                "ascii-only",
                "Restrict to ASCII digit characters (0-9) only.",
                Some('a'),
            )
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: r#"{
    "123": [1, 2],
    "abc": [3, 4],
    "4": [5, 6],
} |
polars into-df --as-columns |
polars select (polars selector digit) |
polars sort-by "123" "4" |
polars collect"#,
                description: "Select columns whose names consist entirely of digits",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "123" => [1i64, 2i64],
                            "4" => [5i64, 6i64],
                        )
                        .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: r#"{
    "123": [1, 2],
    "abc": [3, 4],
} |
polars into-df --as-columns |
polars select (polars selector digit --ascii-only) |
polars collect"#,
                description: "Select digit-named columns using ASCII digits only",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "123" => [1i64, 2i64],
                        )
                        .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["columns", "select", "digit", "numeric", "number"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        mut input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.take_metadata();

        let ascii_only = call.has_flag("ascii-only")?;
        let pattern = if ascii_only {
            "^[0-9]+$".to_string()
        } else {
            r"^\p{Nd}+$".to_string()
        };
        debug!("SelectorDigit: pattern = {pattern}");
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
        test_polars_plugin_command(&SelectorDigit)
    }
}
