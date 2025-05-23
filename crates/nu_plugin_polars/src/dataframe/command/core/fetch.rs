use crate::PolarsPlugin;
use crate::dataframe::values::{Column, NuDataFrame};
use crate::values::{CustomValueSupport, NuLazyFrame};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct LazyFetch;

impl PluginCommand for LazyFetch {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars fetch"
    }

    fn description(&self) -> &str {
        "Collects the lazyframe to the selected rows."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "rows",
                SyntaxShape::Int,
                "number of rows to be fetched from lazyframe",
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Fetch a rows from the dataframe",
            example: "[[a b]; [6 2] [4 2] [2 2]] | polars into-df | polars fetch 2",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_int(6), Value::test_int(4)],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_int(2), Value::test_int(2)],
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
        let rows: i64 = call.req(0)?;
        let value = input.into_value(call.head)?;
        let lazy = NuLazyFrame::try_from_value_coerce(plugin, &value)?;

        let mut eager: NuDataFrame = lazy
            .to_polars()
            .fetch(rows as usize)
            .map_err(|e| ShellError::GenericError {
                error: "Error fetching rows".into(),
                msg: e.to_string(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            })?
            .into();

        // mark this as not from lazy so it doesn't get converted back to a lazy frame
        eager.from_lazy = false;
        eager
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
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&LazyFetch)
    }
}
