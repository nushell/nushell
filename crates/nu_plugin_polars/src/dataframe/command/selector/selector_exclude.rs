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
use polars::prelude::{PlSmallStr, Selector};
use std::sync::Arc;

#[derive(Clone)]
pub struct SelectorExclude;

impl PluginCommand for SelectorExclude {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars selector exclude"
    }

    fn description(&self) -> &str {
        "Select all columns except those with the given name(s). This is the inverse of `polars selector by-name`."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "column",
                SyntaxShape::String,
                "Column name(s) to exclude from the selection.",
            )
            .input_output_type(Type::Any, PolarsPluginType::NuSelector.into())
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: r#"{
    "a": [1.0, 2.0],
    "b": [3.0, 4.0],
    "c": [5, 6],
} |
polars into-df --as-columns |
polars select (polars selector exclude a b) |
polars collect"#,
                description: "Select all columns except 'a' and 'b'",
                result: Some(
                    NuDataFrame::from(
                        df!(
                            "c" => [5i64, 6i64],
                        )
                        .expect("simple df for test should not fail"),
                    )
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                example: r#"[[a b c]; [1 2 3] [4 5 6]]
    | polars into-df
    | polars select (polars selector exclude c)
    | polars collect"#,
                description: "Select all columns except 'c'",
                result: None,
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["columns", "select", "exclude", "except", "drop"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        mut input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.take_metadata();
        let columns: Vec<String> = call.rest(0)?;

        if columns.is_empty() {
            return Err(LabeledError::new("Missing column names")
                .with_label("At least one column name is required", call.head));
        }

        let names: Arc<[PlSmallStr]> = columns
            .into_iter()
            .map(PlSmallStr::from)
            .collect::<Vec<_>>()
            .into();

        let excluded = Selector::ByName {
            names,
            strict: false,
        };
        let selector = Selector::Difference(Arc::new(Selector::Wildcard), Arc::new(excluded));
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
        test_polars_plugin_command(&SelectorExclude)
    }
}
