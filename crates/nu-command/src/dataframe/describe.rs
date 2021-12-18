use super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span,
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
        "dfr describe"
    }

    fn usage(&self) -> &str {
        "Describes dataframes numeric columns"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "dataframe description",
            example: "[[a b]; [1 1] [1 1]] | dfr to-df | dfr describe",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "descriptor".to_string(),
                        vec![
                            "count".to_string().into(),
                            "sum".to_string().into(),
                            "mean".to_string().into(),
                            "median".to_string().into(),
                            "std".to_string().into(),
                            "min".to_string().into(),
                            "25%".to_string().into(),
                            "50%".to_string().into(),
                            "75%".to_string().into(),
                            "max".to_string().into(),
                        ],
                    ),
                    Column::new(
                        "a (i64)".to_string(),
                        vec![
                            2.0.into(),
                            2.0.into(),
                            1.0.into(),
                            1.0.into(),
                            0.0.into(),
                            1.0.into(),
                            1.0.into(),
                            1.0.into(),
                            1.0.into(),
                            1.0.into(),
                        ],
                    ),
                    Column::new(
                        "b (i64)".to_string(),
                        vec![
                            2.0.into(),
                            2.0.into(),
                            1.0.into(),
                            1.0.into(),
                            0.0.into(),
                            1.0.into(),
                            1.0.into(),
                            1.0.into(),
                            1.0.into(),
                            1.0.into(),
                        ],
                    ),
                ])
                .expect("simple df for test should not fail")
                .into_value(Span::unknown()),
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
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

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

            let std = match col.std_as_series().get(0) {
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

            let q_25 = col
                .quantile_as_series(0.25)
                .ok()
                .and_then(|ca| ca.cast(&DataType::Float64).ok())
                .and_then(|ca| match ca.get(0) {
                    AnyValue::Float64(v) => Some(v),
                    _ => None,
                });

            let q_50 = col
                .quantile_as_series(0.50)
                .ok()
                .and_then(|ca| ca.cast(&DataType::Float64).ok())
                .and_then(|ca| match ca.get(0) {
                    AnyValue::Float64(v) => Some(v),
                    _ => None,
                });

            let q_75 = col
                .quantile_as_series(0.75)
                .ok()
                .and_then(|ca| ca.cast(&DataType::Float64).ok())
                .and_then(|ca| match ca.get(0) {
                    AnyValue::Float64(v) => Some(v),
                    _ => None,
                });

            let max = col
                .max_as_series()
                .cast(&DataType::Float64)
                .ok()
                .and_then(|ca| match ca.get(0) {
                    AnyValue::Float64(v) => Some(v),
                    _ => None,
                });

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

    DataFrame::new(res)
        .map_err(|e| {
            ShellError::SpannedLabeledError("Dataframe Error".into(), e.to_string(), call.head)
        })
        .map(|df| PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(DescribeDF {})])
    }
}
