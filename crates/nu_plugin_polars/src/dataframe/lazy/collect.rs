use crate::{
    dataframe::values::{Column, NuDataFrame},
    Cacheable, CustomValueSupport, PolarsPlugin,
};

use super::super::values::NuLazyFrame;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, Span, Type, Value};

#[derive(Clone)]
pub struct LazyCollect;

impl PluginCommand for LazyCollect {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars collect"
    }

    fn usage(&self) -> &str {
        "Collect lazy dataframe into eager dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "drop duplicates",
            example: "[[a b]; [1 2] [3 4]] | polars into-lazy | polars collect",
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
                .base_value(Span::test_data())
                .expect("rendering base value should not fail"),
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
        let lazy = NuLazyFrame::try_from_pipeline(plugin, input, call.head)?;
        let eager = lazy.collect(call.head)?;
        let value = eager
            .cache(plugin, engine)
            .map_err(LabeledError::from)?
            .into_value(call.head);

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
