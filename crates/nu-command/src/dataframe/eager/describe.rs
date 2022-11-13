use super::super::values::{Column, NuDataFrame};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use polars::{
    chunked_array::ChunkedArray,
    prelude::{
        AnyValue, DataFrame, DataType, Float64Type, IntoSeries, NewChunkedArray,
        QuantileInterpolOptions, Series, Utf8Type,
    },
};

#[derive(Clone)]
pub struct DescribeDF;

impl Command for DescribeDF {
    fn name(&self) -> &str {
        "describe"
    }

    fn usage(&self) -> &str {
        "Describes dataframes numeric columns"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Custom("dataframe".into()))
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .named(
                "quantiles",
                SyntaxShape::Table,
                "optional quantiles for describe",
                Some('q'),
            )
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "dataframe description",
            example: "[[a b]; [1 1] [1 1]] | into df | describe",
            result: Some(
                NuDataFrame::try_from_columns(vec![
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
                ])
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
            .map(|value| match value {
                Value::Float { val, span } => {
                    if (&0.0..=&1.0).contains(&val) {
                        Ok(*val)
                    } else {
                        Err(ShellError::GenericError(
                            "Incorrect value for quantile".to_string(),
                            "value should be between 0 and 1".to_string(),
                            Some(*span),
                            None,
                            Vec::new(),
                        ))
                    }
                }
                _ => match value.span() {
                    Ok(span) => Err(ShellError::GenericError(
                        "Incorrect value for quantile".to_string(),
                        "value should be a float".to_string(),
                        Some(span),
                        None,
                        Vec::new(),
                    )),
                    Err(e) => Err(e),
                },
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

    let names = ChunkedArray::<Utf8Type>::from_slice_options("descriptor", &labels).into_series();

    let head = std::iter::once(names);

    let tail = df
        .as_ref()
        .get_columns()
        .iter()
        .filter(|col| col.dtype() != &DataType::Object("object"))
        .map(|col| {
            let count = col.len() as f64;

            let sum = col
                .sum_as_series()
                .cast(&DataType::Float64)
                .ok()
                .and_then(|ca| match ca.get(0) {
                    AnyValue::Float64(v) => Some(v),
                    _ => None,
                });

            let mean = match col.mean_as_series().get(0) {
                AnyValue::Float64(v) => Some(v),
                _ => None,
            };

            let median = match col.median_as_series().get(0) {
                AnyValue::Float64(v) => Some(v),
                _ => None,
            };

            let std = match col.std_as_series(0).get(0) {
                AnyValue::Float64(v) => Some(v),
                _ => None,
            };

            let min = col
                .min_as_series()
                .cast(&DataType::Float64)
                .ok()
                .and_then(|ca| match ca.get(0) {
                    AnyValue::Float64(v) => Some(v),
                    _ => None,
                });

            let mut quantiles = quantiles
                .clone()
                .into_iter()
                .map(|q| {
                    col.quantile_as_series(q, QuantileInterpolOptions::default())
                        .ok()
                        .and_then(|ca| ca.cast(&DataType::Float64).ok())
                        .and_then(|ca| match ca.get(0) {
                            AnyValue::Float64(v) => Some(v),
                            _ => None,
                        })
                })
                .collect::<Vec<Option<f64>>>();

            let max = col
                .max_as_series()
                .cast(&DataType::Float64)
                .ok()
                .and_then(|ca| match ca.get(0) {
                    AnyValue::Float64(v) => Some(v),
                    _ => None,
                });

            let mut descriptors = vec![Some(count), sum, mean, median, std, min];
            descriptors.append(&mut quantiles);
            descriptors.push(max);

            let name = format!("{} ({})", col.name(), col.dtype());
            ChunkedArray::<Float64Type>::from_slice_options(&name, &descriptors).into_series()
        });

    let res = head.chain(tail).collect::<Vec<Series>>();

    DataFrame::new(res)
        .map_err(|e| {
            ShellError::GenericError(
                "Dataframe Error".into(),
                e.to_string(),
                Some(call.head),
                None,
                Vec::new(),
            )
        })
        .map(|df| PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(DescribeDF {})])
    }
}
