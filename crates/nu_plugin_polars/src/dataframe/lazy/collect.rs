use crate::{
    dataframe::values::{Column, NuDataFrame},
    PolarsDataFramePlugin,
};

use super::super::values::NuLazyFrame;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, LabeledError, PipelineData, PluginExample, PluginSignature, Span, Type, Value,
};

#[derive(Clone)]
pub struct LazyCollect;

impl PluginCommand for LazyCollect {
    type Plugin = PolarsDataFramePlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("polars collect")
            .usage("Collect lazy dataframe into eager dataframe.")
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("lazyframe".into()))
            .plugin_examples(vec![PluginExample {
                description: "drop duplicates".into(),
                example: "[[a b]; [1 2] [3 4]] | polars into-lazy | polars collect".into(),
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(1), Value::test_int(3)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(2), Value::test_int(4)],
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
        _plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?;
        let eager = lazy.collect(call.head)?;
        let value = Value::custom_value(Box::new(eager.custom_value()), call.head);

        Ok(PipelineData::Value(value, None))
    }
}

// todo - fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![Box::new(LazyCollect {})])
//     }
// }
