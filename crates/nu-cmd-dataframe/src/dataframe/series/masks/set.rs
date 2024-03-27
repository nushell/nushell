use crate::dataframe::values::{Column, NuDataFrame};
use nu_engine::command_prelude::*;

use polars::prelude::{ChunkSet, DataType, IntoSeries};

#[derive(Clone)]
pub struct SetSeries;

impl Command for SetSeries {
    fn name(&self) -> &str {
        "dfr set"
    }

    fn usage(&self) -> &str {
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
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Shifts the values by a given period",
            example: r#"let s = ([1 2 2 3 3] | dfr into-df | dfr shift 2);
    let mask = ($s | dfr is-null);
    $s | dfr set 0 --mask $mask"#,
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
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        command(engine_state, stack, call, input)
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let value: Value = call.req(engine_state, stack, 0)?;

    let mask_value: Value = call
        .get_flag(engine_state, stack, "mask")?
        .expect("required named value");
    let mask_span = mask_value.span();
    let mask = NuDataFrame::try_from_value(mask_value)?.as_series(mask_span)?;

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

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;
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

            NuDataFrame::try_from_series(vec![res.into_series()], call.head)
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

            NuDataFrame::try_from_series(vec![res.into_series()], call.head)
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
            res.rename("string");

            NuDataFrame::try_from_series(vec![res.into_series()], call.head)
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
    };

    res.map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::super::test_dataframe::test_dataframe;
    use super::super::super::{IsNull, Shift};
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![
            Box::new(SetSeries {}),
            Box::new(IsNull {}),
            Box::new(Shift {}),
        ])
    }
}
