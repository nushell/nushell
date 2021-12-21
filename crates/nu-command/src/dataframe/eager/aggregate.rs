use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    did_you_mean,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};
use polars::{frame::groupby::GroupBy, prelude::PolarsError};

use crate::dataframe::values::NuGroupBy;

use super::super::values::{Column, NuDataFrame};

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
        name: &Spanned<String>,
        quantile: Option<Spanned<f64>>,
    ) -> Result<Operation, ShellError> {
        match name.item.as_ref() {
            "mean" => Ok(Operation::Mean),
            "sum" => Ok(Operation::Sum),
            "min" => Ok(Operation::Min),
            "max" => Ok(Operation::Max),
            "first" => Ok(Operation::First),
            "last" => Ok(Operation::Last),
            "nunique" => Ok(Operation::Nunique),
            "quantile" => match quantile {
                None => Err(ShellError::SpannedLabeledError(
                    "Quantile value not fount".into(),
                    "Quantile operation requires quantile value".into(),
                    name.span,
                )),
                Some(value) => {
                    if (value.item < 0.0) | (value.item > 1.0) {
                        Err(ShellError::SpannedLabeledError(
                            "Inappropriate quantile".into(),
                            "Quantile value should be between 0.0 and 1.0".into(),
                            value.span,
                        ))
                    } else {
                        Ok(Operation::Quantile(value.item))
                    }
                }
            },
            "median" => Ok(Operation::Median),
            "var" => Ok(Operation::Var),
            "std" => Ok(Operation::Std),
            "count" => Ok(Operation::Count),
            selection => {
                let possibilities = [
                    "mean".to_string(),
                    "sum".to_string(),
                    "min".to_string(),
                    "max".to_string(),
                    "first".to_string(),
                    "last".to_string(),
                    "nunique".to_string(),
                    "quantile".to_string(),
                    "median".to_string(),
                    "var".to_string(),
                    "std".to_string(),
                    "count".to_string(),
                ];

                match did_you_mean(&possibilities, selection) {
                    Some(suggestion) => Err(ShellError::DidYouMean(suggestion, name.span)),
                    None => Err(ShellError::SpannedLabeledErrorHelp(
                        "Operation not fount".into(),
                        "Operation does not exist".into(),
                        name.span,
                        "Perhaps you want: mean, sum, min, max, first, last, nunique, quantile, median, var, std, or count".into(),
                    ))
                }
            }
        }
    }

    fn to_str(&self) -> &'static str {
        match self {
            Self::Mean => "mean",
            Self::Sum => "sum",
            Self::Min => "min",
            Self::Max => "max",
            Self::First => "first",
            Self::Last => "last",
            Self::Nunique => "nunique",
            Self::Quantile(_) => "quantile",
            Self::Median => "median",
            Self::Var => "var",
            Self::Std => "std",
            Self::Count => "count",
        }
    }
}

#[derive(Clone)]
pub struct Aggregate;

impl Command for Aggregate {
    fn name(&self) -> &str {
        "dfr aggregate"
    }

    fn usage(&self) -> &str {
        "Performs an aggregation operation on a dataframe and groupby object"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "operation-name",
                SyntaxShape::String,
                "\n\tDataframes: mean, sum, min, max, quantile, median, var, std
\tGroupBy: mean, sum, min, max, first, last, nunique, quantile, median, var, std, count",
            )
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
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Aggregate sum by grouping by column a and summing on col b",
                example:
                    "[[a b]; [one 1] [one 2]] | dfr to-df | dfr group-by a | dfr aggregate sum",
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new("a".to_string(), vec![Value::test_string("one")]),
                        Column::new("b".to_string(), vec![Value::test_int(3)]),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Aggregate sum in dataframe columns",
                example: "[[a b]; [4 1] [5 2]] | dfr to-df | dfr aggregate sum",
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new("a".to_string(), vec![Value::test_int(9)]),
                        Column::new("b".to_string(), vec![Value::test_int(3)]),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Aggregate sum in series",
                example: "[4 1 5 6] | dfr to-df | dfr aggregate sum",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "0".to_string(),
                        vec![Value::test_int(16)],
                    )])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
        ]
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
    let operation: Spanned<String> = call.req(engine_state, stack, 0)?;
    let quantile: Option<Spanned<f64>> = call.get_flag(engine_state, stack, "quantile")?;
    let op = Operation::from_tagged(&operation, quantile)?;

    match input {
        PipelineData::Value(Value::CustomValue { val, span }, _) => {
            let df = val.as_any().downcast_ref::<NuDataFrame>();
            let groupby = val.as_any().downcast_ref::<NuGroupBy>();

            match (df, groupby) {
                (Some(df), None) => {
                    let df = df.as_ref();
                    let res = perform_dataframe_aggregation(df, op, operation.span)?;

                    Ok(PipelineData::Value(
                        NuDataFrame::dataframe_into_value(res, span),
                        None,
                    ))
                }
                (None, Some(nu_groupby)) => {
                    let groupby = nu_groupby.to_groupby()?;

                    let res = perform_groupby_aggregation(
                        groupby,
                        op,
                        operation.span,
                        call.head,
                        call.has_flag("explicit"),
                    )?;

                    Ok(PipelineData::Value(
                        NuDataFrame::dataframe_into_value(res, span),
                        None,
                    ))
                }
                _ => Err(ShellError::SpannedLabeledError(
                    "Incorrect datatype".into(),
                    "no groupby or dataframe found in input stream".into(),
                    call.head,
                )),
            }
        }
        _ => Err(ShellError::SpannedLabeledError(
            "Incorrect datatype".into(),
            "no groupby or dataframe found in input stream".into(),
            call.head,
        )),
    }
}

fn perform_groupby_aggregation(
    groupby: GroupBy,
    operation: Operation,
    operation_span: Span,
    agg_span: Span,
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
            _ => operation_span,
        };

        ShellError::SpannedLabeledError("Error calculating aggregation".into(), e.to_string(), span)
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
    operation_span: Span,
) -> Result<polars::prelude::DataFrame, ShellError> {
    match operation {
        Operation::Mean => Ok(dataframe.mean()),
        Operation::Sum => Ok(dataframe.sum()),
        Operation::Min => Ok(dataframe.min()),
        Operation::Max => Ok(dataframe.max()),
        Operation::Quantile(quantile) => dataframe.quantile(quantile).map_err(|e| {
            ShellError::SpannedLabeledError(
                "Error calculating quantile".into(),
                e.to_string(),
                operation_span,
            )
        }),
        Operation::Median => Ok(dataframe.median()),
        Operation::Var => Ok(dataframe.var()),
        Operation::Std => Ok(dataframe.std()),
        operation => {
            let possibilities = [
                "mean".to_string(),
                "sum".to_string(),
                "min".to_string(),
                "max".to_string(),
                "quantile".to_string(),
                "median".to_string(),
                "var".to_string(),
                "std".to_string(),
            ];

            match did_you_mean(&possibilities, operation.to_str()) {
                Some(suggestion) => Err(ShellError::DidYouMean(suggestion, operation_span)),
                None => Err(ShellError::SpannedLabeledErrorHelp(
                    "Operation not fount".into(),
                    "Operation does not exist".into(),
                    operation_span,
                    "Perhaps you want: mean, sum, min, max, quantile, median, var, or std".into(),
                )),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::super::CreateGroupBy;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(Aggregate {}), Box::new(CreateGroupBy {})])
    }
}
