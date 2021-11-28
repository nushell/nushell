use super::objects::nu_dataframe::NuDataFrame;

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature,
};
use polars::{
    chunked_array::ChunkedArray,
    prelude::{
        AnyValue, DataFrame, DataType, Float64Type, IntoSeries, NewChunkedArray, Series, Utf8Type,
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
        Signature::build(self.name().to_string()).category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "dataframe description",
            example: "[[a b]; [1 1] [1 1]] | to-df | describe",
            result: None,
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
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline(input, call.head.clone())?;

    let names = ChunkedArray::<Utf8Type>::new_from_opt_slice(
        "descriptor",
        &[
            Some("count"),
            Some("sum"),
            Some("mean"),
            Some("median"),
            Some("std"),
            Some("min"),
            Some("25%"),
            Some("50%"),
            Some("75%"),
            Some("max"),
        ],
    )
    .into_series();

    let head = std::iter::once(names);

    let tail = df
        .as_ref()
        .get_columns()
        .iter()
        .filter(|col| col.dtype() != &DataType::Object("object"))
        .map(|col| {
            let count = col.len() as f64;

            let sum = match col.sum_as_series().cast(&DataType::Float64) {
                Ok(ca) => match ca.get(0) {
                    AnyValue::Float64(v) => Some(v),
                    _ => None,
                },
                Err(_) => None,
            };

            let mean = match col.mean_as_series().get(0) {
                AnyValue::Float64(v) => Some(v),
                _ => None,
            };

            let median = match col.median_as_series().get(0) {
                AnyValue::Float64(v) => Some(v),
                _ => None,
            };

            let std = match col.std_as_series().get(0) {
                AnyValue::Float64(v) => Some(v),
                _ => None,
            };

            let min = match col.min_as_series().cast(&DataType::Float64) {
                Ok(ca) => match ca.get(0) {
                    AnyValue::Float64(v) => Some(v),
                    _ => None,
                },
                Err(_) => None,
            };

            let q_25 = match col.quantile_as_series(0.25) {
                Ok(ca) => match ca.cast(&DataType::Float64) {
                    Ok(ca) => match ca.get(0) {
                        AnyValue::Float64(v) => Some(v),
                        _ => None,
                    },
                    Err(_) => None,
                },
                Err(_) => None,
            };

            let q_50 = match col.quantile_as_series(0.50) {
                Ok(ca) => match ca.cast(&DataType::Float64) {
                    Ok(ca) => match ca.get(0) {
                        AnyValue::Float64(v) => Some(v),
                        _ => None,
                    },
                    Err(_) => None,
                },
                Err(_) => None,
            };

            let q_75 = match col.quantile_as_series(0.75) {
                Ok(ca) => match ca.cast(&DataType::Float64) {
                    Ok(ca) => match ca.get(0) {
                        AnyValue::Float64(v) => Some(v),
                        _ => None,
                    },
                    Err(_) => None,
                },
                Err(_) => None,
            };

            let max = match col.max_as_series().cast(&DataType::Float64) {
                Ok(ca) => match ca.get(0) {
                    AnyValue::Float64(v) => Some(v),
                    _ => None,
                },
                Err(_) => None,
            };

            let name = format!("{} ({})", col.name(), col.dtype());
            ChunkedArray::<Float64Type>::new_from_opt_slice(
                &name,
                &[
                    Some(count),
                    sum,
                    mean,
                    median,
                    std,
                    min,
                    q_25,
                    q_50,
                    q_75,
                    max,
                ],
            )
            .into_series()
        });

    let res = head.chain(tail).collect::<Vec<Series>>();
    let df = DataFrame::new(res).map_err(|e| {
        ShellError::LabeledError("Dataframe Error".into(), e.to_string(), call.head)
    })?;
    Ok(PipelineData::Value(NuDataFrame::dataframe_into_value(
        df, call.head,
    )))
}
