use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, Span, Type};
use polars::df;

use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, NuDataFrame, NuLazyFrame},
};

pub struct LazyCache;

impl PluginCommand for LazyCache {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars cache"
    }

    fn description(&self) -> &str {
        "Caches operations in a new LazyFrame."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Caches the result into a new LazyFrame",
            example: "[[a b]; [6 2] [4 2] [2 2]] | polars into-df 
                | polars reverse 
                | polars cache
                | polars sort-by a",
            result: Some(
                NuDataFrame::from(
                    df!(
                        "a" => [2i64, 4, 6],
                        "b" => [2i64, 2, 2],
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
        let lazy = NuLazyFrame::try_from_pipeline_coerce(plugin, input, call.head)
            .map_err(LabeledError::from)?;
        let lazy = NuLazyFrame::new(lazy.from_eager, lazy.to_polars().cache());
        lazy.to_pipeline_data(plugin, engine, call.head)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;
    use nu_protocol::ShellError;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&LazyCache)
    }
}
