use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use polars::prelude::DataType;

use crate::{PolarsPlugin, dataframe::values::Column, values::CustomValueSupport};

use crate::values::{NuDataFrame, PolarsPluginType};

#[derive(Clone)]
pub struct TakeDF;

impl PluginCommand for TakeDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars take"
    }

    fn description(&self) -> &str {
        "Creates new dataframe using the given indices."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "indices",
                SyntaxShape::Any,
                "list of indices used to take data",
            )
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
        vec![
            Example {
                description: "Takes selected rows from dataframe",
                example: r#"let df = ([[a b]; [4 1] [5 2] [4 3]] | polars into-df);
    let indices = ([0 2] | polars into-df);
    $df | polars take $indices"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(4), Value::test_int(4)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(1), Value::test_int(3)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Takes selected rows from series",
                example: r#"let series = ([4 1 5 2 4 3] | polars into-df);
    let indices = ([0 2] | polars into-df);
    $series | polars take $indices"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "0".to_string(),
                            vec![Value::test_int(4), Value::test_int(5)],
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
    let index_value: Value = call.req(0)?;
    let index_span = index_value.span();
    let index = NuDataFrame::try_from_value_coerce(plugin, &index_value, call.head)?
        .as_series(index_span)?;

    let casted = match index.dtype() {
        DataType::UInt32 | DataType::UInt64 | DataType::Int32 | DataType::Int64 => index
            .cast(&DataType::UInt64)
            .map_err(|e| ShellError::GenericError {
                error: "Error casting index list".into(),
                msg: e.to_string(),
                span: Some(index_span),
                help: None,
                inner: vec![],
            }),
        _ => Err(ShellError::GenericError {
            error: "Incorrect type".into(),
            msg: "Series with incorrect type".into(),
            span: Some(call.head),
            help: Some("Consider using a Series with type int type".into()),
            inner: vec![],
        }),
    }?;

    let indices = casted.u64().map_err(|e| ShellError::GenericError {
        error: "Error casting index list".into(),
        msg: e.to_string(),
        span: Some(index_span),
        help: None,
        inner: vec![],
    })?;

    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
    let polars_df = df
        .to_polars()
        .take(indices)
        .map_err(|e| ShellError::GenericError {
            error: "Error taking values".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?;

    let df = NuDataFrame::new(df.from_lazy, polars_df);
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&TakeDF)
    }
}
