use crate::{
    dataframe::values::{Column, NuDataFrame},
    values::{cant_convert_err, CustomValueSupport, PolarsPluginObject, PolarsPluginType},
    Cacheable, PolarsPlugin,
};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, Span, Type, Value};

#[derive(Clone)]
pub struct LazyCollect;

impl PluginCommand for LazyCollect {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars collect"
    }

    fn description(&self) -> &str {
        "Collect lazy dataframe into eager dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(
                Type::Custom("NuDataFrame".into()),
                Type::Custom("NuDataFrame".into()),
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
        let value = input.into_value(call.head)?;
        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuLazyFrame(lazy) => {
                let mut eager = lazy.collect(call.head)?;
                // We don't want this converted back to a lazy frame
                eager.from_lazy = true;
                Ok(PipelineData::Value(
                    eager
                        .cache(plugin, engine, call.head)?
                        .into_value(call.head),
                    None,
                ))
            }
            PolarsPluginObject::NuDataFrame(df) => {
                // This should just increment the cache value.
                // We can return a value back without incrementing the
                // cache value or the value will be dropped (issue #12828)
                let _ = plugin.cache.get(&df.id, true)?;

                // just return the dataframe, add to cache again to be safe
                Ok(PipelineData::Value(
                    df.cache(plugin, engine, call.head)?.into_value(call.head),
                    None,
                ))
            }
            _ => Err(cant_convert_err(
                &value,
                &[PolarsPluginType::NuLazyFrame, PolarsPluginType::NuDataFrame],
            )),
        }
        .map_err(LabeledError::from)
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
