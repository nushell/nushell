use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{dataframe::NuDataFrame, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use polars::frame::groupby::GroupBy;

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
                "Perhaps you want: mean, sum, min, max, first, last, nunique, quantile, median, count",
                &name.tag,
            )),
        }
    }
}

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe groupby"
    }

    fn usage(&self) -> &str {
        "Creates a groupby operation on a dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe groupby")
            .required("columns", SyntaxShape::Table, "groupby columns")
            .required(
                "aggregation columns",
                SyntaxShape::Table,
                "columns to perform aggregation",
            )
            .required("operation", SyntaxShape::String, "aggregate operation")
            .named(
                "quantile",
                SyntaxShape::Number,
                "auantile value for quantile operation",
                Some('q'),
            )
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        groupby(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "",
            example: "",
            result: None,
        }]
    }
}

fn groupby(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let mut args = args.evaluate_once()?;

    let quantile: Option<Tagged<f64>> = args.get_flag("quantile")?;
    let operation: Tagged<String> = args.req(2)?;
    let op = Operation::from_tagged(&operation, quantile)?;

    // Extracting the names of the columns to perform the groupby
    let columns: Vec<Value> = args.req(0)?;
    let (columns_string, col_span) = convert_columns(&columns, &tag)?;

    // Extracting the names of the columns to perform the aggregation
    let agg_cols: Vec<Value> = args.req(1)?;
    let (aggregation_string, agg_span) = convert_columns(&agg_cols, &tag)?;

    // The operation is only done in one dataframe. Only one input is
    // expected from the InputStream
    match args.input.next() {
        None => Err(ShellError::labeled_error(
            "No input received",
            "missing dataframe input from stream",
            &tag,
        )),
        Some(value) => {
            if let UntaggedValue::DataFrame(NuDataFrame {
                dataframe: Some(df),
                ..
            }) = value.value
            {
                let groupby = df
                    .groupby(columns_string)
                    .map_err(|e| {
                        ShellError::labeled_error("Groupby error", format!("{}", e), col_span)
                    })?
                    .select(aggregation_string);

                let res = perform_aggregation(groupby, op, &operation.tag, &agg_span)?;

                let final_df = Value {
                    tag,
                    value: UntaggedValue::DataFrame(NuDataFrame {
                        dataframe: Some(res),
                        name: "agg result".to_string(),
                    }),
                };

                Ok(OutputStream::one(final_df))
            } else {
                Err(ShellError::labeled_error(
                    "No dataframe in stream",
                    "no dataframe found in input stream",
                    &tag,
                ))
            }
        }
    }
}

fn perform_aggregation(
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
        let span = if e.to_string().contains("Not found") {
            agg_span
        } else {
            &operation_tag.span
        };

        ShellError::labeled_error("Aggregation error", format!("{}", e), span)
    })
}
