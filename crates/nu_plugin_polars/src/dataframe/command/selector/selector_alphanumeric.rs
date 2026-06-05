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
pub struct SelectorAlphanumeric;

impl PluginCommand for SelectorAlphanumeric {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector alphanumeric"
    }

    fn description(&self) -> &str {
        r#"Select all columns with alphanumeric names (eg: only letters). Matching column names cannot contain *any* non-alphanumeric characters. Note that the definition of "alphanumeric" consists of all valid Unicode alphanumeric characters by default; this can be changed by setting `ascii_only=true`."#
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("ascii-only", "Indicate whether to consider only ASCII alphanumeric characters, or the full Unicode range of valid letters (accented, idiographic, etc).", Some('a'))
            .switch("ignore-spaces", "Indicate whether to ignore the presence of spaces in column names; if so, only the other (non-space) characters are considered. ", Some('s'))
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: r#"
                {
                    "1st_col": [100, 200, 300],
                    "flagged": [true, false, true],
                    "00prefix": ["01:aa", "02:bb", "03:cc"],
                    "last col": ["x", "y", "z"],
                } |
                polars into-df --as-columns |
                polars select (polars selector alphanumeric) |
                polars sort-by 00prefix flagged |
                polars collect
                "#,
                description: "Select columns with alphanumeric names.",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "flagged" => [true, false, true],
                            "00prefix" => ["01:aa", "02:bb", "03:cc"],
                        )
                        .expect("Failed to create expected DataFrame"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: r#"
                {
                    "1st_col": [100, 200, 300],
                    "flagged": [true, false, true],
                    "00prefix": ["01:aa", "02:bb", "03:cc"],
                    "last col": ["x", "y", "z"],
                } |
                polars into-df --as-columns |
                polars select (polars selector alphanumeric --ignore-spaces) |
                polars sort-by 00prefix 'last col' flagged |
                polars collect
                "#,
                description: "Select columns with alphanumeric names ignoring spaces.",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "flagged" => [true, false, true],
                            "00prefix" => ["01:aa", "02:bb", "03:cc"],
                            "last col" => ["x", "y", "z"],
                        )
                        .expect("Failed to create expected DataFrame"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: r#"
                {
                    "1st_col": [100, 200, 300],
                    "flagged": [true, false, true],
                    "00prefix": ["01:aa", "02:bb", "03:cc"],
                    "last col": ["x", "y", "z"],
                } |
                polars into-df --as-columns |
                polars select (polars selector alphanumeric | polars selector not) |
                polars sort-by '1st_col' 'last col' |
                polars collect
                "#,
                description: r#"Select all columns *except* for those with alphanumeric names."#,
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "1st_col" => [100, 200, 300],
                            "last col" => ["x", "y", "z"],
                        )
                        .expect("Failed to create expected DataFrame"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: r#"
                {
                    "1st_col": [100, 200, 300],
                    "flagged": [true, false, true],
                    "00prefix": ["01:aa", "02:bb", "03:cc"],
                    "last col": ["x", "y", "z"],
                } |
                polars into-df --as-columns |
                polars select (polars selector alphanumeric --ignore-spaces | polars selector not) |
                polars collect
                "#,
                description: r#"Select all columns *except* for those with alphanumeric names, ignoring spaces."#,
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "1st_col" => [100, 200, 300],
                        )
                        .expect("Failed to create expected DataFrame"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["columns", "select", "alphanumeric"]
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
        let re_digit = if ascii_only { "0-9" } else { r"\d" };
        let re_alpha = if ascii_only {
            "a-zA-Z"
        } else {
            r"\p{Alphabetic}"
        };
        let re_space = if ignore_spaces { " " } else { "" };
        let pattern = format!("^[{re_alpha}{re_digit}{re_space}]+$");
        debug!("SelectorAlphanumeric: pattern = {pattern}");
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
        test_polars_plugin_command(&SelectorAlphanumeric)
    }
}
