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
pub struct SelectorAlpha;

impl PluginCommand for SelectorAlpha {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector alpha"
    }

    fn description(&self) -> &str {
        r#"Select all columns with alphabetic names (eg: only letters). Matching column names cannot contain *any* non-alphabetic characters. Note that the definition of "alphabetic" consists of all valid Unicode alphabetic characters by default; this can be changed by setting `--ascii-only`."#
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("ascii-only", "Indicate whether to consider only ASCII alphabetic characters, or the full Unicode range of valid letters (accented, idiographic, etc).", Some('a'))
            .switch("ignore-spaces", "Indicate whether to ignore the presence of spaces in column names; if so, only the other (non-space) characters are considered. ", Some('s'))
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: r#"
                {
                    "no1": [100, 200, 300],
                    "café": ["espresso", "latte", "mocha"],
                    "t or f": [true, false, null],
                    "hmm": ["aaa", "bbb", "ccc"],
                    "都市": ["東京", "大阪", "京都"],
                } |
                polars into-df --as-columns |
                polars select (polars selector alpha) |
                polars sort-by  café hmm 都市 |
                polars collect
                "#,
                description: "Select columns with alphabetic names; note that accented characters and kanji are recognised as alphabetic here.",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "café" => ["espresso", "latte", "mocha"],
                            "hmm" => ["aaa", "bbb", "ccc"],
                            "都市" => ["東京", "大阪", "京都"],
                        )
                        .expect("Failed to create expected DataFrame"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: r#"
                {
                    "no1": [100, 200, 300],
                    "café": ["espresso", "latte", "mocha"],
                    "t or f": [true, false, null],
                    "hmm": ["aaa", "bbb", "ccc"],
                    "都市": ["東京", "大阪", "京都"],
                } |
                polars into-df --as-columns |
                polars select (polars selector alpha --ascii-only) |
                polars collect
                "#,
                description: r#"Constrain the definition of "alphabetic" to ASCII characters only."#,
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "hmm" => ["aaa", "bbb", "ccc"],
                        )
                        .expect("Failed to create expected DataFrame"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: r#"
                {
                    "no1": [100, 200, 300],
                    "café": ["espresso", "latte", "mocha"],
                    "t or f": [true, false, null],
                    "hmm": ["aaa", "bbb", "ccc"],
                    "都市": ["東京", "大阪", "京都"],
                } |
                polars into-df --as-columns |
                polars select (polars selector alpha --ascii-only --ignore-spaces) |
                polars sort-by  "t or f" hmm |
                polars collect
                "#,
                description: r#"Constrain the definition of "alphabetic" to ASCII characters only and ignore whitespace."#,
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "t or f" => [None, Some(false), Some(true)],
                            "hmm" => ["ccc", "bbb", "aaa"],
                        )
                        .expect("Failed to create expected DataFrame"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: r#"
                {
                    "no1": [100, 200, 300],
                    "café": ["espresso", "latte", "mocha"],
                    "t or f": [true, false, null],
                    "hmm": ["aaa", "bbb", "ccc"],
                    "都市": ["東京", "大阪", "京都"],
                } |
                polars into-df --as-columns |
                polars select (polars selector alpha | polars selector not) |
                polars sort-by no1 "t or f"|
                polars collect
                "#,
                description: r#"Select all columns *except* for those with alphabetic names."#,
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "no1" => [100, 200, 300],
                            "t or f" => [Some(true), Some(false), None],
                        )
                        .expect("Failed to create expected DataFrame"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: r#"
                {
                    "no1": [100, 200, 300],
                    "café": ["espresso", "latte", "mocha"],
                    "t or f": [true, false, null],
                    "hmm": ["aaa", "bbb", "ccc"],
                    "都市": ["東京", "大阪", "京都"],
                } |
                polars into-df --as-columns |
                polars select (polars selector alpha --ignore-spaces | polars selector not) |
                polars sort-by no1 |
                polars collect
                "#,
                description: r#"Select all columns *except* for those with alphabetic names and do not have spaces."#,
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "no1" => [100, 200, 300],
                        )
                        .expect("Failed to create expected DataFrame"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["columns", "select", "alpha"]
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
        let ignore_spaces = call.has_flag("ignore-spaces")?;
        let re_alpha = if ascii_only {
            "a-zA-Z"
        } else {
            r"\p{Alphabetic}"
        };
        let re_space = if ignore_spaces { " " } else { "" };
        let pattern = format!("^[{re_alpha}{re_space}]+$");
        debug!("SelectorAlpha: pattern = {pattern}");
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
        test_polars_plugin_command(&SelectorAlpha)
    }
}
