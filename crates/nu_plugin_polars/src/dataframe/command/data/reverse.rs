use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, Span, Type, Value};

use crate::{
    PolarsPlugin,
    values::{Column, CustomValueSupport, NuDataFrame, NuLazyFrame},
};

pub struct LazyReverse;

impl PluginCommand for LazyReverse {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars reverse"
    }

    fn description(&self) -> &str {
        "Reverses the LazyFrame"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Reverses the dataframe.",
            example: "[[a b]; [6 2] [4 2] [2 2]] | polars into-df | polars reverse",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_int(2), Value::test_int(4), Value::test_int(6)],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_int(2), Value::test_int(2), Value::test_int(2)],
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
        let metadata = input.metadata();
        let lazy = NuLazyFrame::try_from_pipeline_coerce(plugin, input, call.head)
            .map_err(LabeledError::from)?;
        let lazy = NuLazyFrame::new(lazy.from_eager, lazy.to_polars().reverse());
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
        test_polars_plugin_command(&LazyReverse)
    }
}
