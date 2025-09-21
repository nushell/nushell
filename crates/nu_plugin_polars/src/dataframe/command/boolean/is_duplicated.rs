use crate::{PolarsPlugin, values::CustomValueSupport};

use super::super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type, Value,
};
use polars::prelude::IntoSeries;

#[derive(Clone)]
pub struct IsDuplicated;

impl PluginCommand for IsDuplicated {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars is-duplicated"
    }

    fn description(&self) -> &str {
        "Creates mask indicating duplicated values."
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
        vec![
            Example {
                description: "Create mask indicating duplicated values",
                example: "[5 6 6 6 8 8 8] | polars into-df | polars is-duplicated",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "is_duplicated".to_string(),
                            vec![
                                Value::test_bool(false),
                                Value::test_bool(true),
                                Value::test_bool(true),
                                Value::test_bool(true),
                                Value::test_bool(true),
                                Value::test_bool(true),
                                Value::test_bool(true),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Create mask indicating duplicated rows in a dataframe",
                example: "[[a, b]; [1 2] [1 2] [3 3] [3 3] [1 1]] | polars into-df | polars is-duplicated",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "is_duplicated".to_string(),
                            vec![
                                Value::test_bool(true),
                                Value::test_bool(true),
                                Value::test_bool(true),
                                Value::test_bool(true),
                                Value::test_bool(false),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
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
    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;

    let mut res = df
        .as_ref()
        .is_duplicated()
        .map_err(|e| ShellError::GenericError {
            error: "Error finding duplicates".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?
        .into_series();

    res.rename("is_duplicated".into());

    let df = NuDataFrame::try_from_series_vec(vec![res], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&IsDuplicated)
    }
}
