use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::df;
use polars::prelude::UniqueKeepStrategy;

use crate::PolarsPlugin;
use crate::values::CustomValueSupport;

use crate::values::NuDataFrame;
use crate::values::utils::convert_columns_string;

#[derive(Clone)]
pub struct DropDuplicates;

impl PluginCommand for DropDuplicates {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars drop-duplicates"
    }

    fn description(&self) -> &str {
        "Drops duplicate values in dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional(
                "subset",
                SyntaxShape::Table(vec![]),
                "subset of columns to drop duplicates",
            )
            .switch("maintain", "maintain order", Some('m'))
            .switch(
                "last",
                "keeps last duplicate value (by default keeps first)",
                Some('l'),
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "drop duplicates",
            example: "[[a b]; [1 2] [3 4] [1 2]] | polars into-df
                | polars drop-duplicates
                | polars sort-by a",
            result: Some(
                NuDataFrame::from(
                    df!(
                        "a" => &[1i64, 3],
                        "b" => &[2i64, 4],
                    )
                    .expect("should not fail"),
                )
                .into_value(Span::test_data()),
            ),
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        command(plugin, engine, call, input)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let columns: Option<Vec<Value>> = call.opt(0)?;
    let (subset, col_span) = match columns {
        Some(cols) => {
            let (agg_string, col_span) = convert_columns_string(cols, call.head)?;
            (Some(agg_string), col_span)
        }
        None => (None, call.head),
    };

    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;

    let subset_slice = subset.as_ref().map(|cols| &cols[..]);

    let keep_strategy = if call.has_flag("last")? {
        UniqueKeepStrategy::Last
    } else {
        UniqueKeepStrategy::First
    };

    let polars_df = df
        .as_ref()
        .unique_stable(subset_slice, keep_strategy, None)
        .map_err(|e| ShellError::GenericError {
            error: "Error dropping duplicates".into(),
            msg: e.to_string(),
            span: Some(col_span),
            help: None,
            inner: vec![],
        })?;

    let df = NuDataFrame::new(df.from_lazy, polars_df);
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use crate::test::test_polars_plugin_command;

    use super::*;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&DropDuplicates)
    }
}
