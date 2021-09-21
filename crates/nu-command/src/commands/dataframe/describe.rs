use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, NuDataFrame},
    Signature, UntaggedValue,
};
use polars::{
    chunked_array::ChunkedArray,
    prelude::{
        AnyValue, DataFrame as PolarsDF, DataType, Float64Type, IntoSeries, NewChunkedArray,
        Series, Utf8Type,
    },
};

use super::utils::parse_polars_error;

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe describe"
    }

    fn usage(&self) -> &str {
        "[DataFrame] Describes dataframes numeric columns"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe describe")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Describes dataframe",
            example: "[[a b]; [1 1] [1 1]] | dataframe to-df | dataframe describe",
            result: Some(vec![NuDataFrame::try_from_columns(
                vec![
                    Column::new(
                        "descriptor".to_string(),
                        vec![
                            UntaggedValue::string("count").into(),
                            UntaggedValue::string("sum").into(),
                            UntaggedValue::string("mean").into(),
                            UntaggedValue::string("median").into(),
                            UntaggedValue::string("std").into(),
                            UntaggedValue::string("min").into(),
                            UntaggedValue::string("25%").into(),
                            UntaggedValue::string("50%").into(),
                            UntaggedValue::string("75%").into(),
                            UntaggedValue::string("max").into(),
                        ],
                    ),
                    Column::new(
                        "a (i64)".to_string(),
                        vec![
                            UntaggedValue::decimal_from_float(2.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(2.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(1.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(1.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(0.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(1.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(1.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(1.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(1.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(1.0, Span::default()).into(),
                        ],
                    ),
                    Column::new(
                        "b (i64)".to_string(),
                        vec![
                            UntaggedValue::decimal_from_float(2.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(2.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(1.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(1.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(0.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(1.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(1.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(1.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(1.0, Span::default()).into(),
                            UntaggedValue::decimal_from_float(1.0, Span::default()).into(),
                        ],
                    ),
                ],
                &Span::default(),
            )
            .expect("simple df for test should not fail")
            .into_value(Tag::default())]),
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let (df, _) = NuDataFrame::try_from_stream(&mut args.input, &tag.span)?;

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

    let tail = df.as_ref().get_columns().iter().map(|col| {
        let count = col.len() as f64;

        let sum = match col.sum_as_series().cast_with_dtype(&DataType::Float64) {
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

        let min = match col.min_as_series().cast_with_dtype(&DataType::Float64) {
            Ok(ca) => match ca.get(0) {
                AnyValue::Float64(v) => Some(v),
                _ => None,
            },
            Err(_) => None,
        };

        let q_25 = match col.quantile_as_series(0.25) {
            Ok(ca) => match ca.cast_with_dtype(&DataType::Float64) {
                Ok(ca) => match ca.get(0) {
                    AnyValue::Float64(v) => Some(v),
                    _ => None,
                },
                Err(_) => None,
            },
            Err(_) => None,
        };

        let q_50 = match col.quantile_as_series(0.50) {
            Ok(ca) => match ca.cast_with_dtype(&DataType::Float64) {
                Ok(ca) => match ca.get(0) {
                    AnyValue::Float64(v) => Some(v),
                    _ => None,
                },
                Err(_) => None,
            },
            Err(_) => None,
        };

        let q_75 = match col.quantile_as_series(0.75) {
            Ok(ca) => match ca.cast_with_dtype(&DataType::Float64) {
                Ok(ca) => match ca.get(0) {
                    AnyValue::Float64(v) => Some(v),
                    _ => None,
                },
                Err(_) => None,
            },
            Err(_) => None,
        };

        let max = match col.max_as_series().cast_with_dtype(&DataType::Float64) {
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
    let df = PolarsDF::new(res).map_err(|e| parse_polars_error::<&str>(&e, &tag.span, None))?;
    let df = NuDataFrame::dataframe_to_value(df, tag);
    Ok(OutputStream::one(df))
}

#[cfg(test)]
mod tests {
    use super::DataFrame;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test_dataframe as test_examples;

        test_examples(DataFrame {})
    }
}
