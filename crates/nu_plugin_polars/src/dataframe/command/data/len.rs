use crate::{
    PolarsPlugin,
    values::{Column, CustomValueSupport, NuDataFrame, NuExpression, PolarsPluginType},
};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, Signature, Span, Type, Value};

#[derive(Clone)]
pub struct ExprLen;

impl PluginCommand for ExprLen {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars len"
    }

    fn description(&self) -> &str {
        "Return the number of rows in the context. This is similar to COUNT(*) in SQL."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Any, PolarsPluginType::NuExpression.into())
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Count the number of rows in the the dataframe.",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars select (polars len) | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new("len".to_string(), vec![Value::test_int(2)])],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Creates a last expression from a column",
                example: "polars col a | polars last",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let res: NuExpression = polars::prelude::len().into();
        res.to_pipeline_data(plugin, engine, call.head)
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
        test_polars_plugin_command(&ExprLen)
    }
}
