use crate::dataframe::values::{Column, NuDataFrame};
use nu_engine::command_prelude::*;

use polars::{
    chunked_array::ChunkedArray,
    prelude::{
        AnyValue, DataFrame, DataType, Float64Type, IntoSeries, NewChunkedArray,
        QuantileInterpolOptions, Series, StringType,
    },
};

#[derive(Clone)]
pub struct Summary;

impl Command for Summary {
    fn name(&self) -> &str {
        "dfr summary"
    }

    fn usage(&self) -> &str {
        "For a dataframe, produces descriptive statistics (summary statistics) for its numeric columns."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Custom("dataframe".into()))
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .named(
                "quantiles",
                SyntaxShape::Table(vec![]),
                "provide optional quantiles",
                Some('q'),
            )
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "list dataframe descriptives",
            example: "[[a b]; [1 1] [1 1]] | dfr into-df | dfr summary",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "descriptor".to_string(),
                            vec![
                                Value::test_string("count"),
                                Value::test_string("sum"),
                                Value::test_string("mean"),
                                Value::test_string("median"),
                                Value::test_string("std"),
                                Value::test_string("min"),
                                Value::test_string("25%"),
                                Value::test_string("50%"),
                                Value::test_string("75%"),
                                Value::test_string("max"),
                            ],
                        ),
                        Column::new(
                            "a (i64)".to_string(),
                            vec![
                                Value::test_float(2.0),
                                Value::test_float(2.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                                Value::test_float(0.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                            ],
                        ),
                        Column::new(
                            "b (i64)".to_string(),
                            vec![
                                Value::test_float(2.0),
                                Value::test_float(2.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                                Value::test_float(0.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                                Value::test_float(1.0),
                            ],
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
    let quantiles: Option<Vec<Value>> = call.get_flag(engine_state, stack, "quantiles")?;
    let quantiles = quantiles.map(|values| {
        values
            .iter()
            .map(|value| {
                let span = value.span();
                match value {
                    Value::Float { val, .. } => {
                        if (&0.0..=&1.0).contains(&val) {
                            Ok(*val)
                        } else {
                            Err(ShellError::GenericError {
                                error: "Incorrect value for quantile".into(),
                                msg: "value should be between 0 and 1".into(),
                                span: Some(span),
                                help: None,
                                inner: vec![],
                            })
                        }
                    }
                    Value::Error { error, .. } => Err(*error.clone()),
                    _ => Err(ShellError::GenericError {
                        error: "Incorrect value for quantile".into(),
                        msg: "value should be a float".into(),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    }),
                }
            })
            .collect::<Result<Vec<f64>, ShellError>>()
    });

    let quantiles = match quantiles {
        Some(quantiles) => quantiles?,
        None => vec![0.25, 0.50, 0.75],
    };

    let mut quantiles_labels = quantiles
        .iter()
        .map(|q| Some(format!("{}%", q * 100.0)))
        .collect::<Vec<Option<String>>>();
    let mut labels = vec![
        Some("count".to_string()),
        Some("sum".to_string()),
        Some("mean".to_string()),
        Some("median".to_string()),
        Some("std".to_string()),
        Some("min".to_string()),
    ];
    labels.append(&mut quantiles_labels);
    labels.push(Some("max".to_string()));

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let names = ChunkedArray::<StringType>::from_slice_options("descriptor", &labels).into_series();

    let head = std::iter::once(names);

    let tail = df
        .as_ref()
        .get_columns()
        .iter()
        .filter(|col| !matches!(col.dtype(), &DataType::Object("object", _)))
        .map(|col| {
            let count = col.len() as f64;

            let sum = col.sum_as_series().ok().and_then(|series| {
                series
                    .cast(&DataType::Float64)
                    .ok()
                    .and_then(|ca| match ca.get(0) {
                        Ok(AnyValue::Float64(v)) => Some(v),
                        _ => None,
                    })
            });

            let mean = match col.mean_as_series().get(0) {
                Ok(AnyValue::Float64(v)) => Some(v),
                _ => None,
            };

            let median = match col.median_as_series() {
                Ok(v) => match v.get(0) {
                    Ok(AnyValue::Float64(v)) => Some(v),
                    _ => None,
                },
                _ => None,
            };

            let std = match col.std_as_series(0) {
                Ok(v) => match v.get(0) {
                    Ok(AnyValue::Float64(v)) => Some(v),
                    _ => None,
                },
                _ => None,
            };

            let min = col.min_as_series().ok().and_then(|series| {
                series
                    .cast(&DataType::Float64)
                    .ok()
                    .and_then(|ca| match ca.get(0) {
                        Ok(AnyValue::Float64(v)) => Some(v),
                        _ => None,
                    })
            });

            let mut quantiles = quantiles
                .clone()
                .into_iter()
                .map(|q| {
                    col.quantile_as_series(q, QuantileInterpolOptions::default())
                        .ok()
                        .and_then(|ca| ca.cast(&DataType::Float64).ok())
                        .and_then(|ca| match ca.get(0) {
                            Ok(AnyValue::Float64(v)) => Some(v),
                            _ => None,
                        })
                })
                .collect::<Vec<Option<f64>>>();

            let max = col.max_as_series().ok().and_then(|series| {
                series
                    .cast(&DataType::Float64)
                    .ok()
                    .and_then(|ca| match ca.get(0) {
                        Ok(AnyValue::Float64(v)) => Some(v),
                        _ => None,
                    })
            });

            let mut descriptors = vec![Some(count), sum, mean, median, std, min];
            descriptors.append(&mut quantiles);
            descriptors.push(max);

            let name = format!("{} ({})", col.name(), col.dtype());
            ChunkedArray::<Float64Type>::from_slice_options(&name, &descriptors).into_series()
        });

    let res = head.chain(tail).collect::<Vec<Series>>();

    DataFrame::new(res)
        .map_err(|e| ShellError::GenericError {
            error: "Dataframe Error".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })
        .map(|df| PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(Summary {})])
    }
}
