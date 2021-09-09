use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{NuDataFrame, NuGroupBy},
    Signature, SyntaxShape,
};
use nu_source::Tagged;

use polars::prelude::DataType;

enum Operation {
    First,
    Sum,
    Min,
    Max,
    Mean,
    Median,
}

impl Operation {
    fn from_tagged(name: &Tagged<String>) -> Result<Operation, ShellError> {
        match name.item.as_ref() {
            "first" => Ok(Operation::First),
            "sum" => Ok(Operation::Sum),
            "min" => Ok(Operation::Min),
            "max" => Ok(Operation::Max),
            "mean" => Ok(Operation::Mean),
            "median" => Ok(Operation::Median),
            _ => Err(ShellError::labeled_error_with_secondary(
                "Operation not fount",
                "Operation does not exist for pivot",
                &name.tag,
                "Perhaps you want: first, sum, min, max, mean, median",
                &name.tag,
            )),
        }
    }
}

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe pivot"
    }

    fn usage(&self) -> &str {
        "[GroupBy] Performs a pivot operation on a groupby object"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe pivot")
            .required(
                "pivot column",
                SyntaxShape::String,
                "pivot column to perform pivot",
            )
            .required(
                "value column",
                SyntaxShape::String,
                "value column to perform pivot",
            )
            .required("operation", SyntaxShape::String, "aggregate operation")
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Pivot a dataframe on b and aggregation on col c",
            example:
                "[[a b c]; [one x 1] [two y 2]] | dataframe to-df | dataframe group-by a | dataframe pivot b c sum",
            result: None, // No sample because there are nulls in the result dataframe
        }]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    // Extracting the pivot col from arguments
    let pivot_col: Tagged<String> = args.req(0)?;

    // Extracting the value col from arguments
    let value_col: Tagged<String> = args.req(1)?;

    let operation: Tagged<String> = args.req(2)?;
    let op = Operation::from_tagged(&operation)?;

    // The operation is only done in one groupby. Only one input is
    // expected from the InputStream
    let nu_groupby = NuGroupBy::try_from_stream(&mut args.input, &tag.span)?;
    let df_ref = nu_groupby.as_ref();

    check_pivot_column(df_ref, &pivot_col)?;
    check_value_column(df_ref, &value_col)?;

    let mut groupby = nu_groupby.to_groupby()?;

    let pivot = groupby.pivot(&pivot_col.item, &value_col.item);

    let res = match op {
        Operation::Mean => pivot.mean(),
        Operation::Sum => pivot.sum(),
        Operation::Min => pivot.min(),
        Operation::Max => pivot.max(),
        Operation::First => pivot.first(),
        Operation::Median => pivot.median(),
    }
    .map_err(|e| parse_polars_error::<&str>(&e, &tag.span, None))?;

    Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
}

fn check_pivot_column(
    df: &polars::prelude::DataFrame,
    col: &Tagged<String>,
) -> Result<(), ShellError> {
    let series = df
        .column(&col.item)
        .map_err(|e| parse_polars_error::<&str>(&e, &col.tag.span, None))?;

    match series.dtype() {
        DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32
        | DataType::UInt64
        | DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::Int64
        | DataType::Utf8 => Ok(()),
        _ => Err(ShellError::labeled_error(
            "Pivot error",
            format!("Unsupported datatype {}", series.dtype()),
            col.tag.span,
        )),
    }
}

fn check_value_column(
    df: &polars::prelude::DataFrame,
    col: &Tagged<String>,
) -> Result<(), ShellError> {
    let series = df
        .column(&col.item)
        .map_err(|e| parse_polars_error::<&str>(&e, &col.tag.span, None))?;

    match series.dtype() {
        DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32
        | DataType::UInt64
        | DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::Int64
        | DataType::Float32
        | DataType::Float64 => Ok(()),
        _ => Err(ShellError::labeled_error(
            "Pivot error",
            format!("Unsupported datatype {}", series.dtype()),
            col.tag.span,
        )),
    }
}
