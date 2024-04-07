use crate::{
    dataframe::values::{Column, NuDataFrame},
    values::CustomValueSupport,
    Cacheable, PolarsPlugin,
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
        let lazy = NuLazyFrame::try_from_pipeline(plugin, input, call.head)?;
        let eager = lazy.collect(call.head)?;
        Ok(PipelineData::Value(
            eager.cache(plugin, engine)?.into_value(call.head),
            None,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), nu_protocol::ShellError> {
        test_polars_plugin_command(&LazyCollect)
    }
}
