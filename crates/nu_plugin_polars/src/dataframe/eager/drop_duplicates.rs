use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, LabeledError, PipelineData, PluginExample, PluginSignature, ShellError, Span,
    SyntaxShape, Type, Value,
};
use polars::prelude::UniqueKeepStrategy;

use crate::{Cacheable, CustomValueSupport, PolarsPlugin};

use super::super::values::utils::convert_columns_string;
use super::super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct DropDuplicates;

impl PluginCommand for DropDuplicates {
    type Plugin = PolarsPlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("polars drop-duplicates")
            .usage("Drops duplicate values in dataframe.")
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
            .plugin_examples(vec![PluginExample {
                description: "drop duplicates".into(),
                example: "[[a b]; [1 2] [3 4] [1 2]] | polars into-df | polars drop-duplicates"
                    .into(),
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(3), Value::test_int(1)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(4), Value::test_int(2)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            }])
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        command(plugin, engine, call, input).map_err(LabeledError::from)
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

    let df = NuDataFrame::try_from_pipeline(plugin, input, call.head)?;

    let subset_slice = subset.as_ref().map(|cols| &cols[..]);

    let keep_strategy = if call.has_flag("last")? {
        UniqueKeepStrategy::Last
    } else {
        UniqueKeepStrategy::First
    };

    let polars_df = df
        .as_ref()
        .unique(subset_slice, keep_strategy, None)
        .map_err(|e| ShellError::GenericError {
            error: "Error dropping duplicates".into(),
            msg: e.to_string(),
            span: Some(col_span),
            help: None,
            inner: vec![],
        })?;

    let df = NuDataFrame::new(false, polars_df);
    let val = df.cache(plugin, engine)?.into_value(call.head);
    Ok(PipelineData::Value(val, None))
}

// todo - fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![Box::new(DropDuplicates {})])
//     }
// }
