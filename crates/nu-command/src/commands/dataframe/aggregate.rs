use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, PolarsData},
    Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value,
};
use nu_source::Tagged;
use polars::{
    frame::groupby::GroupBy,
    prelude::{DataType, PolarsError, Series},
};

use super::utils::convert_columns;

enum Operation {
    Mean,
    Sum,
    Min,
    Max,
    First,
    Last,
    Nunique,
    Quantile(f64),
    Median,
    Var,
    Std,
    Count,
}

impl Operation {
    fn from_tagged(
        name: &Tagged<String>,
        quantile: Option<Tagged<f64>>,
    ) -> Result<Operation, ShellError> {
        match name.item.as_ref() {
            "mean" => Ok(Operation::Mean),
            "sum" => Ok(Operation::Sum),
            "min" => Ok(Operation::Min),
            "max" => Ok(Operation::Max),
            "first" => Ok(Operation::First),
            "last" => Ok(Operation::Last),
            "nunique" => Ok(Operation::Nunique),
            "quantile" => {
                match quantile {
                    None => Err(ShellError::labeled_error(
                        "Quantile value not fount",
                        "Quantile operation requires quantile value",
                        &name.tag,
                    )),
                Some(value ) => {
                    if (value.item < 0.0) | (value.item > 1.0) {
                        Err(ShellError::labeled_error(
                            "Inappropriate quantile",
                            "Quantile value should be between 0.0 and 1.0",
                            &value.tag,
                        ))
                    } else {
                        Ok(Operation::Quantile(value.item))
                    }
                }
                }
            }
            "median" => Ok(Operation::Median),
            "var" => Ok(Operation::Var),
            "std" => Ok(Operation::Std),
            "count" => Ok(Operation::Count),
            _ => Err(ShellError::labeled_error_with_secondary(
                "Operation not fount",
                "Operation does not exist",
                &name.tag,
                "Perhaps you want: mean, sum, min, max, first, last, nunique, quantile, median, var, std, or count",
                &name.tag,
            )),
        }
    }
}

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe aggregate"
    }

    fn usage(&self) -> &str {
        "Performs an aggregation operation on a dataframe or groupby object"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe aggregate")
            .required("operation", SyntaxShape::String, "aggregate operation")
            .optional(
                "selection",
                SyntaxShape::Table,
                "columns to perform aggregation",
            )
            .named(
                "quantile",
                SyntaxShape::Number,
                "quantile value for quantile operation",
                Some('q'),
            )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Aggregate sum by grouping by column a and summing on col b",
                example:
                    "[[a b]; [one 1] [one 2]] | dataframe to-df | dataframe group-by [a] | dataframe aggregate sum",
                result: None,
            },
            Example {
                description: "Aggregate sum in dataframe columns",
                example: "[[a b]; [4 1] [5 2]] | dataframe to-df | dataframe aggregate sum",
                result: None,
            },
            Example {
                description: "Aggregate sum in series",
                example: "[4 1 5 6] | dataframe to-series | dataframe aggregate sum",
                result: None,
            },
        ]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let quantile: Option<Tagged<f64>> = args.get_flag("quantile")?;
    let operation: Tagged<String> = args.req(0)?;
    let op = Operation::from_tagged(&operation, quantile)?;

    // Extracting the selection columns of the columns to perform the aggregation
    let agg_cols: Option<Vec<Value>> = args.opt(1)?;
    let (selection, agg_span) = match agg_cols {
        Some(cols) => {
            let (agg_string, agg_span) = convert_columns(&cols, &tag)?;
            (Some(agg_string), agg_span)
        }
        None => (None, Span::unknown()),
    };

    let value = args.input.next().ok_or_else(|| {
        ShellError::labeled_error("Empty stream", "No value found in the stream", &tag)
    })?;

    match value.value {
        UntaggedValue::DataFrame(PolarsData::GroupBy(nu_groupby)) => {
            let groupby = nu_groupby.to_groupby()?;

            let groupby = match &selection {
                Some(cols) => groupby.select(cols),
                None => groupby,
            };

            let res = perform_groupby_aggregation(groupby, op, &operation.tag, &agg_span)?;

            Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
        }
        UntaggedValue::DataFrame(PolarsData::EagerDataFrame(df)) => {
            let df = df.as_ref();

            let res = match &selection {
                Some(cols) => {
                    let df = df
                        .select(cols)
                        .map_err(|e| parse_polars_error::<&str>(&e, &agg_span, None))?;

                    perform_dataframe_aggregation(&df, op, &operation.tag)
                }
                None => perform_dataframe_aggregation(&df, op, &operation.tag),
            }?;

            Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
        }
        UntaggedValue::DataFrame(PolarsData::Series(series)) => {
            let value = perform_series_aggregation(series.as_ref(), op, &operation.tag)?;

            Ok(OutputStream::one(value))
        }
        _ => Err(ShellError::labeled_error(
            "No groupby or dataframe",
            "no groupby or found in input stream",
            &value.tag.span,
        )),
    }
}

fn perform_groupby_aggregation(
    groupby: GroupBy,
    operation: Operation,
    operation_tag: &Tag,
    agg_span: &Span,
) -> Result<polars::prelude::DataFrame, ShellError> {
    match operation {
        Operation::Mean => groupby.mean(),
        Operation::Sum => groupby.sum(),
        Operation::Min => groupby.min(),
        Operation::Max => groupby.max(),
        Operation::First => groupby.first(),
        Operation::Last => groupby.last(),
        Operation::Nunique => groupby.n_unique(),
        Operation::Quantile(quantile) => groupby.quantile(quantile),
        Operation::Median => groupby.median(),
        Operation::Var => groupby.var(),
        Operation::Std => groupby.std(),
        Operation::Count => groupby.count(),
    }
    .map_err(|e| {
        let span = match &e {
            PolarsError::NotFound(_) => agg_span,
            _ => &operation_tag.span,
        };

        parse_polars_error::<&str>(&e, span, None)
    })
}

fn perform_dataframe_aggregation(
    dataframe: &polars::prelude::DataFrame,
    operation: Operation,
    operation_tag: &Tag,
) -> Result<polars::prelude::DataFrame, ShellError> {
    match operation {
        Operation::Mean => Ok(dataframe.mean()),
        Operation::Sum => Ok(dataframe.sum()),
        Operation::Min => Ok(dataframe.min()),
        Operation::Max => Ok(dataframe.max()),
        Operation::Quantile(quantile) => dataframe
            .quantile(quantile)
            .map_err(|e| parse_polars_error::<&str>(&e, &operation_tag.span, None)),
        Operation::Median => Ok(dataframe.median()),
        Operation::Var => Ok(dataframe.var()),
        Operation::Std => Ok(dataframe.std()),
        _ => Err(ShellError::labeled_error_with_secondary(
            "Not valid operation",
            "operation not valid for dataframe",
            &operation_tag.span,
            "Perhaps you want: mean, sum, min, max, quantile, median, var, or std",
            &operation_tag.span,
        )),
    }
}

fn perform_series_aggregation(
    series: &Series,
    operation: Operation,
    operation_tag: &Tag,
) -> Result<Value, ShellError> {
    match operation {
        Operation::Mean => {
            let res = match series.mean() {
                Some(val) => UntaggedValue::Primitive(val.into()),
                None => UntaggedValue::Primitive(0.into()),
            };

            let value = Value {
                value: res,
                tag: operation_tag.clone(),
            };

            let mut data = TaggedDictBuilder::new(operation_tag.clone());
            data.insert_value("mean", value);

            Ok(data.into_value())
        }
        Operation::Median => {
            let res = match series.median() {
                Some(val) => UntaggedValue::Primitive(val.into()),
                None => UntaggedValue::Primitive(0.into()),
            };

            let value = Value {
                value: res,
                tag: operation_tag.clone(),
            };

            let mut data = TaggedDictBuilder::new(operation_tag.clone());
            data.insert_value("median", value);

            Ok(data.into_value())
        }
        Operation::Sum => {
            let untagged = match series.dtype() {
                DataType::Int8
                | DataType::Int16
                | DataType::Int32
                | DataType::Int64
                | DataType::UInt8
                | DataType::UInt16
                | DataType::UInt32
                | DataType::UInt64 => {
                    let res: i64 = series.sum().unwrap_or(0);
                    Ok(UntaggedValue::Primitive(res.into()))
                }
                DataType::Float32 | DataType::Float64 => {
                    let res: f64 = series.sum().unwrap_or(0.0);
                    Ok(UntaggedValue::Primitive(res.into()))
                }
                _ => Err(ShellError::labeled_error(
                    "Not valid type",
                    format!(
                        "this operation can not be performed with series of type {}",
                        series.dtype()
                    ),
                    &operation_tag.span,
                )),
            }?;

            let value = Value {
                value: untagged,
                tag: operation_tag.clone(),
            };

            let mut data = TaggedDictBuilder::new(operation_tag.clone());
            data.insert_value("sum", value);

            Ok(data.into_value())
        }
        Operation::Max => {
            let untagged = match series.dtype() {
                DataType::Int8
                | DataType::Int16
                | DataType::Int32
                | DataType::Int64
                | DataType::UInt8
                | DataType::UInt16
                | DataType::UInt32
                | DataType::UInt64 => {
                    let res: i64 = series.max().unwrap_or(0);
                    Ok(UntaggedValue::Primitive(res.into()))
                }
                DataType::Float32 | DataType::Float64 => {
                    let res: f64 = series.max().unwrap_or(0.0);
                    Ok(UntaggedValue::Primitive(res.into()))
                }
                _ => Err(ShellError::labeled_error(
                    "Not valid type",
                    format!(
                        "this operation can not be performed with series of type {}",
                        series.dtype()
                    ),
                    &operation_tag.span,
                )),
            }?;

            let value = Value {
                value: untagged,
                tag: operation_tag.clone(),
            };

            let mut data = TaggedDictBuilder::new(operation_tag.clone());
            data.insert_value("max", value);

            Ok(data.into_value())
        }
        Operation::Min => {
            let untagged = match series.dtype() {
                DataType::Int8
                | DataType::Int16
                | DataType::Int32
                | DataType::Int64
                | DataType::UInt8
                | DataType::UInt16
                | DataType::UInt32
                | DataType::UInt64 => {
                    let res: i64 = series.min().unwrap_or(0);
                    Ok(UntaggedValue::Primitive(res.into()))
                }
                DataType::Float32 | DataType::Float64 => {
                    let res: f64 = series.min().unwrap_or(0.0);
                    Ok(UntaggedValue::Primitive(res.into()))
                }
                _ => Err(ShellError::labeled_error(
                    "Not valid type",
                    format!(
                        "this operation can not be performed with series of type {}",
                        series.dtype()
                    ),
                    &operation_tag.span,
                )),
            }?;

            let value = Value {
                value: untagged,
                tag: operation_tag.clone(),
            };

            let mut data = TaggedDictBuilder::new(operation_tag.clone());
            data.insert_value("min", value);

            Ok(data.into_value())
        }

        _ => Err(ShellError::labeled_error_with_secondary(
            "Not valid operation",
            "operation not valid for series",
            &operation_tag.span,
            "Perhaps you want: mean, median, sum, max, min",
            &operation_tag.span,
        )),
    }
}
