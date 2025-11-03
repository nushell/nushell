use crate::{
    PolarsPlugin, missing_flag_error,
    values::{CustomValueSupport, PolarsPluginType},
};

use super::super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use polars::prelude::{ChunkSet, DataType, IntoSeries};

#[derive(Clone)]
pub struct SetSeries;

impl PluginCommand for SetSeries {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars set"
    }

    fn description(&self) -> &str {
        "Sets value where given mask is true."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("value", SyntaxShape::Any, "value to be inserted in series")
            .required_named(
                "mask",
                SyntaxShape::Any,
                "mask indicating insertions",
                Some('m'),
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
            description: "Shifts the values by a given period",
            example: r#"let s = ([1 2 2 3 3] | polars into-df | polars shift 2);
    let mask = ($s | polars is-null);
    $s | polars set 0 --mask $mask"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "0".to_string(),
                        vec![
                            Value::test_int(0),
                            Value::test_int(0),
                            Value::test_int(1),
                            Value::test_int(2),
                            Value::test_int(2),
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
    let value: Value = call.req(0)?;

    let mask_value: Value = call
        .get_flag("mask")?
        .ok_or_else(|| missing_flag_error("mask", call.head))?;

    let mask_span = mask_value.span();
    let mask =
        NuDataFrame::try_from_value_coerce(plugin, &mask_value, call.head)?.as_series(mask_span)?;

    let bool_mask = match mask.dtype() {
        DataType::Boolean => mask.bool().map_err(|e| ShellError::GenericError {
            error: "Error casting to bool".into(),
            msg: e.to_string(),
            span: Some(mask_span),
            help: None,
            inner: vec![],
        }),
        _ => Err(ShellError::GenericError {
            error: "Incorrect type".into(),
            msg: "can only use bool series as mask".into(),
            span: Some(mask_span),
            help: None,
            inner: vec![],
        }),
    }?;

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

            let res = chunked
                .set(bool_mask, Some(val))
                .map_err(|e| ShellError::GenericError {
                    error: "Error setting value".into(),
                    msg: e.to_string(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
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

            let res = chunked
                .set(bool_mask, Some(val))
                .map_err(|e| ShellError::GenericError {
                    error: "Error setting value".into(),
                    msg: e.to_string(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
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

            let res = chunked.set(bool_mask, Some(val.as_ref())).map_err(|e| {
                ShellError::GenericError {
                    error: "Error setting value".into(),
                    msg: e.to_string(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                }
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
        test_polars_plugin_command(&SetSeries)
    }
}
