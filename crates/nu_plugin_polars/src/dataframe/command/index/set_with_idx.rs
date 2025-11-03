use crate::{
    PolarsPlugin, missing_flag_error,
    values::{CustomValueSupport, PolarsPluginType},
};

use super::super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use polars::{
    chunked_array::cast::CastOptions,
    prelude::{ChunkSet, DataType, IntoSeries},
};

#[derive(Clone)]
pub struct SetWithIndex;

impl PluginCommand for SetWithIndex {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars set-with-idx"
    }

    fn description(&self) -> &str {
        "Sets value in the given index."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("value", SyntaxShape::Any, "value to be inserted in series")
            .required_named(
                "indices",
                SyntaxShape::Any,
                "list of indices indicating where to set the value",
                Some('i'),
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
        vec![Example {
            description: "Set value in selected rows from series",
            example: r#"let series = ([4 1 5 2 4 3] | polars into-df);
    let indices = ([0 2] | polars into-df);
    $series | polars set-with-idx 6 --indices $indices"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "0".to_string(),
                        vec![
                            Value::test_int(6),
                            Value::test_int(1),
                            Value::test_int(6),
                            Value::test_int(2),
                            Value::test_int(4),
                            Value::test_int(3),
                        ],
                    )],
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
    let value: Value = call.req(0)?;

    let indices_value: Value = call
        .get_flag("indices")?
        .ok_or_else(|| missing_flag_error("indices", call.head))?;

    let indices_span = indices_value.span();
    let indices = NuDataFrame::try_from_value_coerce(plugin, &indices_value, call.head)?
        .as_series(indices_span)?;

    let casted = match indices.dtype() {
        DataType::UInt32 | DataType::UInt64 | DataType::Int32 | DataType::Int64 => indices
            .as_ref()
            .cast(&DataType::UInt64, CastOptions::default())
            .map_err(|e| ShellError::GenericError {
                error: "Error casting indices".into(),
                msg: e.to_string(),
                span: Some(indices_span),
                help: None,
                inner: vec![],
            }),
        _ => Err(ShellError::GenericError {
            error: "Incorrect type".into(),
            msg: "Series with incorrect type".into(),
            span: Some(indices_span),
            help: Some("Consider using a Series with type int type".into()),
            inner: vec![],
        }),
    }?;

    let indices = casted
        .u64()
        .map_err(|e| ShellError::GenericError {
            error: "Error casting indices".into(),
            msg: e.to_string(),
            span: Some(indices_span),
            help: None,
            inner: vec![],
        })?
        .into_iter()
        .flatten();

    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
    let series = df.as_series(call.head)?;

    let span = value.span();
    let res = match value {
        Value::Int { val, .. } => {
            let chunked = series.i64().map_err(|e| ShellError::GenericError {
                error: "Error casting to i64".into(),
                msg: e.to_string(),
                span: Some(span),
                help: None,
                inner: vec![],
            })?;

            let res = chunked.scatter_single(indices, Some(val)).map_err(|e| {
                ShellError::GenericError {
                    error: "Error setting value".into(),
                    msg: e.to_string(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                }
            })?;

            NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)
        }
        Value::Float { val, .. } => {
            let chunked = series.f64().map_err(|e| ShellError::GenericError {
                error: "Error casting to f64".into(),
                msg: e.to_string(),
                span: Some(span),
                help: None,
                inner: vec![],
            })?;

            let res = chunked.scatter_single(indices, Some(val)).map_err(|e| {
                ShellError::GenericError {
                    error: "Error setting value".into(),
                    msg: e.to_string(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                }
            })?;

            NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)
        }
        Value::String { val, .. } => {
            let chunked = series.str().map_err(|e| ShellError::GenericError {
                error: "Error casting to string".into(),
                msg: e.to_string(),
                span: Some(span),
                help: None,
                inner: vec![],
            })?;

            let res = chunked
                .scatter_single(indices, Some(val.as_ref()))
                .map_err(|e| ShellError::GenericError {
                    error: "Error setting value".into(),
                    msg: e.to_string(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                })?;

            let mut res = res.into_series();
            res.rename("string".into());

            NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)
        }
        _ => Err(ShellError::GenericError {
            error: "Incorrect value type".into(),
            msg: format!(
                "this value cannot be set in a series of type '{}'",
                series.dtype()
            ),
            span: Some(span),
            help: None,
            inner: vec![],
        }),
    }?;

    res.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&SetWithIndex)
    }
}
