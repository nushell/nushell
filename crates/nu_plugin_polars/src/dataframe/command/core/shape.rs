use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Value,
};

use crate::{PolarsPlugin, dataframe::values::Column, values::CustomValueSupport};

use crate::values::{NuDataFrame, PolarsPluginType};

#[derive(Clone)]
pub struct ShapeDF;

impl PluginCommand for ShapeDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars shape"
    }

    fn description(&self) -> &str {
        "Shows column and row size for a dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (
                    PolarsPluginType::NuDataFrame.into(),
                    PolarsPluginType::NuDataFrame.into(),
                ),
                (
                    PolarsPluginType::NuLazyFrame.into(),
                    PolarsPluginType::NuLazyFrame.into(),
                ),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Shows row and column shape",
            example: "[[a b]; [1 2] [3 4]] | polars into-df | polars shape",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new("rows".to_string(), vec![Value::test_int(2)]),
                        Column::new("columns".to_string(), vec![Value::test_int(2)]),
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
        command(plugin, engine, call, input).map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;

    let rows = Value::int(df.as_ref().height() as i64, call.head);

    let cols = Value::int(df.as_ref().width() as i64, call.head);

    let rows_col = Column::new("rows".to_string(), vec![rows]);
    let cols_col = Column::new("columns".to_string(), vec![cols]);

    let df = NuDataFrame::try_from_columns(vec![rows_col, cols_col], None)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ShapeDF)
    }
}
