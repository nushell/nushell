use crate::{commands::dataframe::utils::parse_polars_error, prelude::*};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::{Column, FrameStruct, NuDataFrame},
    Signature, SyntaxShape, UntaggedValue,
};
use nu_source::Tagged;
use polars::{frame::groupby::GroupBy, prelude::PolarsError};

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
        "[DataFrame, GroupBy, Series] Performs an aggregation operation on a dataframe, groupby or series object"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe aggregate")
            .required("operation", SyntaxShape::String, "aggregate operation")
            .named(
                "quantile",
                SyntaxShape::Number,
                "quantile value for quantile operation",
                Some('q'),
            )
            .switch(
                "explicit",
                "returns explicit names for groupby aggregations",
                Some('e'),
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
                    "[[a b]; [one 1] [one 2]] | dataframe to-df | dataframe group-by a | dataframe aggregate sum",
                result: Some(vec![NuDataFrame::try_from_columns(
                    vec![
                        Column::new("a".to_string(), vec![UntaggedValue::string("one").into()]),
                        Column::new("b".to_string(), vec![UntaggedValue::int(3).into()]),
                    ],
                    &Span::default(),
                )
                .expect("simple df for test should not fail")
                .into_value(Tag::default())]),
            },
            Example {
                description: "Aggregate sum in dataframe columns",
                example: "[[a b]; [4 1] [5 2]] | dataframe to-df | dataframe aggregate sum",
                result: Some(vec![NuDataFrame::try_from_columns(
                    vec![
                        Column::new("a".to_string(), vec![UntaggedValue::int(9).into()]),
                        Column::new("b".to_string(), vec![UntaggedValue::int(3).into()]),
                    ],
                    &Span::default(),
                )
                .expect("simple df for test should not fail")
                .into_value(Tag::default())]),
            },
            Example {
                description: "Aggregate sum in series",
                example: "[4 1 5 6] | dataframe to-df | dataframe aggregate sum",
                result: Some(vec![NuDataFrame::try_from_columns(
                    vec![
                        Column::new("0".to_string(), vec![UntaggedValue::int(16).into()]),
                    ],
                    &Span::default(),
                )
                .expect("simple df for test should not fail")
                .into_value(Tag::default())]),
            },
        ]
    }
}

fn command(mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let quantile: Option<Tagged<f64>> = args.get_flag("quantile")?;
    let operation: Tagged<String> = args.req(0)?;
    let op = Operation::from_tagged(&operation, quantile)?;

    let value = args.input.next().ok_or_else(|| {
        ShellError::labeled_error("Empty stream", "No value found in the stream", &tag)
    })?;

    match value.value {
        UntaggedValue::FrameStruct(FrameStruct::GroupBy(nu_groupby)) => {
            let groupby = nu_groupby.to_groupby()?;

            let res = perform_groupby_aggregation(
                groupby,
                op,
                &operation.tag,
                &tag.span,
                args.has_flag("explicit"),
            )?;

            Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
        }
        UntaggedValue::DataFrame(df) => {
            let df = df.as_ref();

            let res = perform_dataframe_aggregation(df, op, &operation.tag)?;

            Ok(OutputStream::one(NuDataFrame::dataframe_to_value(res, tag)))
        }
        _ => Err(ShellError::labeled_error(
            "No groupby, dataframe or series in stream",
            "no groupby, dataframe or series found in input stream",
            &value.tag.span,
        )),
    }
}

fn perform_groupby_aggregation(
    groupby: GroupBy,
    operation: Operation,
    operation_tag: &Tag,
    agg_span: &Span,
    explicit: bool,
) -> Result<polars::prelude::DataFrame, ShellError> {
    let mut res = match operation {
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
    })?;

    if !explicit {
        let col_names = res
            .get_column_names()
            .iter()
            .map(|name| name.to_string())
            .collect::<Vec<String>>();

        for col in col_names {
            let from = match operation {
                Operation::Mean => "_mean",
                Operation::Sum => "_sum",
                Operation::Min => "_min",
                Operation::Max => "_max",
                Operation::First => "_first",
                Operation::Last => "_last",
                Operation::Nunique => "_n_unique",
                Operation::Quantile(_) => "_quantile",
                Operation::Median => "_median",
                Operation::Var => "_agg_var",
                Operation::Std => "_agg_std",
                Operation::Count => "_count",
            };

            let new_col = match col.find(from) {
                Some(index) => &col[..index],
                None => &col[..],
            };

            res.rename(&col, new_col)
                .expect("Column is always there. Looping with known names");
        }
    }

    Ok(res)
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
