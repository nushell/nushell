use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape,
};
use polars::prelude::DataType;

use crate::dataframe::values::NuGroupBy;

use super::super::values::NuDataFrame;

enum Operation {
    First,
    Sum,
    Min,
    Max,
    Mean,
    Median,
}

impl Operation {
    fn from_tagged(name: Spanned<String>) -> Result<Operation, ShellError> {
        match name.item.as_ref() {
            "first" => Ok(Operation::First),
            "sum" => Ok(Operation::Sum),
            "min" => Ok(Operation::Min),
            "max" => Ok(Operation::Max),
            "mean" => Ok(Operation::Mean),
            "median" => Ok(Operation::Median),
            _ => Err(ShellError::SpannedLabeledErrorHelp(
                "Operation not fount".into(),
                "Operation does not exist for pivot".into(),
                name.span,
                "Options: first, sum, min, max, mean, median".into(),
            )),
        }
    }
}

#[derive(Clone)]
pub struct PivotDF;

impl Command for PivotDF {
    fn name(&self) -> &str {
        "dfr pivot"
    }

    fn usage(&self) -> &str {
        "Performs a pivot operation on a groupby object"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "pivot_column",
                SyntaxShape::String,
                "pivot column to perform pivot",
            )
            .required(
                "value_column",
                SyntaxShape::String,
                "value column to perform pivot",
            )
            .required("operation", SyntaxShape::String, "aggregate operation")
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Pivot a dataframe on b and aggregation on col c",
            example:
                "[[a b c]; [one x 1] [two y 2]] | dfr to-df | dfr group_by a | dfr pivot b c sum",
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
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let pivot_col: Spanned<String> = call.req(engine_state, stack, 0)?;
    let value_col: Spanned<String> = call.req(engine_state, stack, 1)?;
    let operation: Spanned<String> = call.req(engine_state, stack, 2)?;
    let op = Operation::from_tagged(operation)?;

    let nu_groupby = NuGroupBy::try_from_pipeline(input, call.head)?;
    let df_ref = nu_groupby.as_ref();

    check_pivot_column(df_ref, &pivot_col)?;
    check_value_column(df_ref, &value_col)?;

    let mut groupby = nu_groupby.to_groupby()?;

    let pivot = groupby.pivot(&pivot_col.item, &value_col.item);

    match op {
        Operation::Mean => pivot.mean(),
        Operation::Sum => pivot.sum(),
        Operation::Min => pivot.min(),
        Operation::Max => pivot.max(),
        Operation::First => pivot.first(),
        Operation::Median => pivot.median(),
    }
    .map_err(|e| {
        ShellError::SpannedLabeledError("Error creating pivot".into(), e.to_string(), call.head)
    })
    .map(|df| PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None))
}

fn check_pivot_column(
    df: &polars::prelude::DataFrame,
    col: &Spanned<String>,
) -> Result<(), ShellError> {
    let series = df.column(&col.item).map_err(|e| {
        ShellError::SpannedLabeledError("Column not found".into(), e.to_string(), col.span)
    })?;

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
        _ => Err(ShellError::SpannedLabeledError(
            "Pivot error".into(),
            format!("Unsupported datatype {}", series.dtype()),
            col.span,
        )),
    }
}

fn check_value_column(
    df: &polars::prelude::DataFrame,
    col: &Spanned<String>,
) -> Result<(), ShellError> {
    let series = df.column(&col.item).map_err(|e| {
        ShellError::SpannedLabeledError("Column not found".into(), e.to_string(), col.span)
    })?;

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
        _ => Err(ShellError::SpannedLabeledError(
            "Pivot error".into(),
            format!("Unsupported datatype {}", series.dtype()),
            col.span,
        )),
    }
}
