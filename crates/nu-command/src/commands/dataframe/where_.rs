use crate::prelude::*;
use nu_engine::{evaluate_baseline_expr, WholeStreamCommand};
use nu_errors::ShellError;
use nu_protocol::{
    dataframe::NuDataFrame,
    hir::{CapturedBlock, ClassifiedCommand, Expression, Literal, Operator, SpannedExpression},
    Primitive, Signature, SyntaxShape, UnspannedPathMember, UntaggedValue, Value,
};

use super::utils::parse_polars_error;
use polars::prelude::{ChunkCompare, DataType, Series};

pub struct DataFrame;

impl WholeStreamCommand for DataFrame {
    fn name(&self) -> &str {
        "dataframe where"
    }

    fn signature(&self) -> Signature {
        Signature::build("dataframe where").required(
            "condition",
            SyntaxShape::RowCondition,
            "the condition that must match",
        )
    }

    fn usage(&self) -> &str {
        "[DataFrame] Filter dataframe to match the condition"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        command(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Filter dataframe based on column a",
            example: "[[a b]; [1 2] [3 4]] | dataframe to-df | dataframe where a == 1",
            result: None,
        }]
    }
}

fn command(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag.clone();

    let block: CapturedBlock = args.req(0)?;

    let expression = block
        .block
        .block
        .get(0)
        .and_then(|group| {
            group
                .pipelines
                .get(0)
                .and_then(|v| v.list.get(0))
                .and_then(|expr| match &expr {
                    ClassifiedCommand::Expr(expr) => match &expr.as_ref().expr {
                        Expression::Binary(expr) => Some(expr),
                        _ => None,
                    },
                    _ => None,
                })
        })
        .ok_or_else(|| {
            ShellError::labeled_error("Expected a condition", "expected a condition", &tag.span)
        })?;

    let lhs = match &expression.left.expr {
        Expression::FullColumnPath(p) => p.as_ref().tail.get(0),
        _ => None,
    }
    .ok_or_else(|| {
        ShellError::labeled_error(
            "No column name",
            "Not a column name found in left hand side of comparison",
            &expression.left.span,
        )
    })?;

    let (col_name, col_name_span) = match &lhs.unspanned {
        UnspannedPathMember::String(name) => Ok((name, &lhs.span)),
        _ => Err(ShellError::labeled_error(
            "No column name",
            "Not a string as column name",
            &lhs.span,
        )),
    }?;

    let rhs = evaluate_baseline_expr(&expression.right, &args.context)?;

    filter_dataframe(args, &col_name, &col_name_span, &rhs, &expression.op)
}

macro_rules! comparison_arm {
    ($comparison:expr,  $col:expr, $condition:expr, $span:expr) => {
        match $condition {
            Primitive::Int(val) => Ok($comparison($col, *val)),
            Primitive::BigInt(val) => Ok($comparison(
                $col,
                val.to_i64()
                    .expect("Internal error: protocol did not use compatible decimal"),
            )),
            Primitive::Decimal(val) => Ok($comparison(
                $col,
                val.to_f64()
                    .expect("Internal error: protocol did not use compatible decimal"),
            )),
            Primitive::String(val) => {
                let temp: &str = val.as_ref();
                Ok($comparison($col, temp))
            }
            _ => Err(ShellError::labeled_error(
                "Invalid datatype",
                format!(
                    "this operator cannot be used with the selected '{}' datatype",
                    $col.dtype()
                ),
                &$span,
            )),
        }
    };
}

// With the information extracted from the block we can filter the dataframe using
// polars operations
fn filter_dataframe(
    mut args: CommandArgs,
    col_name: &str,
    col_name_span: &Span,
    rhs: &Value,
    operator: &SpannedExpression,
) -> Result<OutputStream, ShellError> {
    let right_condition = match &rhs.value {
        UntaggedValue::Primitive(primitive) => Ok(primitive),
        _ => Err(ShellError::labeled_error(
            "Incorrect argument",
            "Expected primitive values",
            &rhs.tag.span,
        )),
    }?;

    let span = args.call_info.name_tag.span;
    let df = NuDataFrame::try_from_stream(&mut args.input, &span)?;

    let col = df
        .as_ref()
        .column(col_name)
        .map_err(|e| parse_polars_error::<&str>(&e, col_name_span, None))?;

    let op = match &operator.expr {
        Expression::Literal(Literal::Operator(op)) => Ok(op),
        _ => Err(ShellError::labeled_error(
            "Incorrect argument",
            "Expected operator",
            &operator.span,
        )),
    }?;

    let mask = match op {
        Operator::Equal => comparison_arm!(Series::eq, col, right_condition, operator.span),
        Operator::NotEqual => comparison_arm!(Series::neq, col, right_condition, operator.span),
        Operator::LessThan => comparison_arm!(Series::lt, col, right_condition, operator.span),
        Operator::LessThanOrEqual => {
            comparison_arm!(Series::lt_eq, col, right_condition, operator.span)
        }
        Operator::GreaterThan => comparison_arm!(Series::gt, col, right_condition, operator.span),
        Operator::GreaterThanOrEqual => {
            comparison_arm!(Series::gt_eq, col, right_condition, operator.span)
        }
        Operator::Contains => match col.dtype() {
            DataType::Utf8 => match right_condition {
                Primitive::String(pat) => {
                    let casted = col.utf8().map_err(|e| {
                        parse_polars_error::<&str>(&e, &args.call_info.name_tag.span, None)
                    })?;

                    casted.contains(pat).map_err(|e| {
                        parse_polars_error::<&str>(&e, &args.call_info.name_tag.span, None)
                    })
                }
                _ => Err(ShellError::labeled_error_with_secondary(
                    "Incorrect argument",
                    "Can't perform contains with this value",
                    &rhs.tag.span,
                    "Contains only works with strings",
                    &rhs.tag.span,
                )),
            },
            _ => Err(ShellError::labeled_error_with_secondary(
                "Incorrect datatype",
                format!("The selected column is of type '{}'", col.dtype()),
                col_name_span,
                "Perhaps you want to select a column of 'str' type",
                col_name_span,
            )),
        },
        _ => Err(ShellError::labeled_error(
            "Incorrect operator",
            "Not implemented operator for dataframes filter",
            &operator.span,
        )),
    }?;

    let res = df
        .as_ref()
        .filter(&mask)
        .map_err(|e| parse_polars_error::<&str>(&e, &args.call_info.name_tag.span, None))?;

    Ok(OutputStream::one(NuDataFrame::dataframe_to_value(
        res,
        args.call_info.name_tag,
    )))
}
