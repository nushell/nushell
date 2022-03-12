use super::{operations::Axis, NuDataFrame};

use nu_protocol::{ast::Operator, span, ShellError, Span, Spanned, Value};
use num::Zero;
use polars::prelude::{
    BooleanType, ChunkCompare, ChunkedArray, DataType, Float64Type, Int64Type, IntoSeries,
    NumOpsDispatchChecked, PolarsError, Series,
};
use std::ops::{Add, BitAnd, BitOr, Div, Mul, Sub};

pub(super) fn between_dataframes(
    operator: Spanned<Operator>,
    left: &Value,
    lhs: &NuDataFrame,
    right: &Value,
    rhs: &NuDataFrame,
) -> Result<Value, ShellError> {
    let operation_span = span(&[left.span()?, right.span()?]);
    match operator.item {
        Operator::Plus => match lhs.append_df(rhs, Axis::Row, operation_span) {
            Ok(df) => Ok(df.into_value(operation_span)),
            Err(e) => Err(e),
        },
        _ => Err(ShellError::OperatorMismatch {
            op_span: operator.span,
            lhs_ty: left.get_type(),
            lhs_span: left.span()?,
            rhs_ty: right.get_type(),
            rhs_span: right.span()?,
        }),
    }
}

pub(super) fn compute_between_series(
    operator: Spanned<Operator>,
    left: &Value,
    lhs: &Series,
    right: &Value,
    rhs: &Series,
) -> Result<Value, ShellError> {
    let operation_span = span(&[left.span()?, right.span()?]);
    match operator.item {
        Operator::Plus => {
            let mut res = lhs + rhs;
            let name = format!("sum_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            NuDataFrame::series_to_value(res, operation_span)
        }
        Operator::Minus => {
            let mut res = lhs - rhs;
            let name = format!("sub_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            NuDataFrame::series_to_value(res, operation_span)
        }
        Operator::Multiply => {
            let mut res = lhs * rhs;
            let name = format!("mul_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            NuDataFrame::series_to_value(res, operation_span)
        }
        Operator::Divide => {
            let res = lhs.checked_div(rhs);
            match res {
                Ok(mut res) => {
                    let name = format!("div_{}_{}", lhs.name(), rhs.name());
                    res.rename(&name);
                    NuDataFrame::series_to_value(res, operation_span)
                }
                Err(e) => Err(ShellError::SpannedLabeledError(
                    "Division error".into(),
                    e.to_string(),
                    right.span()?,
                )),
            }
        }
        Operator::Equal => {
            let mut res = Series::equal(lhs, rhs).into_series();
            let name = format!("eq_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            NuDataFrame::series_to_value(res, operation_span)
        }
        Operator::NotEqual => {
            let mut res = Series::not_equal(lhs, rhs).into_series();
            let name = format!("neq_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            NuDataFrame::series_to_value(res, operation_span)
        }
        Operator::LessThan => {
            let mut res = Series::lt(lhs, rhs).into_series();
            let name = format!("lt_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            NuDataFrame::series_to_value(res, operation_span)
        }
        Operator::LessThanOrEqual => {
            let mut res = Series::lt_eq(lhs, rhs).into_series();
            let name = format!("lte_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            NuDataFrame::series_to_value(res, operation_span)
        }
        Operator::GreaterThan => {
            let mut res = Series::gt(lhs, rhs).into_series();
            let name = format!("gt_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            NuDataFrame::series_to_value(res, operation_span)
        }
        Operator::GreaterThanOrEqual => {
            let mut res = Series::gt_eq(lhs, rhs).into_series();
            let name = format!("gte_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            NuDataFrame::series_to_value(res, operation_span)
        }
        Operator::And => match lhs.dtype() {
            DataType::Boolean => {
                let lhs_cast = lhs.bool();
                let rhs_cast = rhs.bool();

                match (lhs_cast, rhs_cast) {
                    (Ok(l), Ok(r)) => {
                        let mut res = l.bitand(r).into_series();
                        let name = format!("and_{}_{}", lhs.name(), rhs.name());
                        res.rename(&name);
                        NuDataFrame::series_to_value(res, operation_span)
                    }
                    _ => Err(ShellError::SpannedLabeledError(
                        "Incompatible types".into(),
                        "unable to cast to boolean".into(),
                        right.span()?,
                    )),
                }
            }
            _ => Err(ShellError::IncompatibleParametersSingle(
                format!(
                    "Operation {} can only be done with boolean values",
                    operator.item
                ),
                operation_span,
            )),
        },
        Operator::Or => match lhs.dtype() {
            DataType::Boolean => {
                let lhs_cast = lhs.bool();
                let rhs_cast = rhs.bool();

                match (lhs_cast, rhs_cast) {
                    (Ok(l), Ok(r)) => {
                        let mut res = l.bitor(r).into_series();
                        let name = format!("or_{}_{}", lhs.name(), rhs.name());
                        res.rename(&name);
                        NuDataFrame::series_to_value(res, operation_span)
                    }
                    _ => Err(ShellError::SpannedLabeledError(
                        "Incompatible types".into(),
                        "unable to cast to boolean".into(),
                        right.span()?,
                    )),
                }
            }
            _ => Err(ShellError::IncompatibleParametersSingle(
                format!(
                    "Operation {} can only be done with boolean values",
                    operator.item
                ),
                operation_span,
            )),
        },
        _ => Err(ShellError::OperatorMismatch {
            op_span: operator.span,
            lhs_ty: left.get_type(),
            lhs_span: left.span()?,
            rhs_ty: right.get_type(),
            rhs_span: right.span()?,
        }),
    }
}

pub(super) fn compute_series_single_value(
    operator: Spanned<Operator>,
    left: &Value,
    lhs: &NuDataFrame,
    right: &Value,
) -> Result<Value, ShellError> {
    if !lhs.is_series() {
        return Err(ShellError::OperatorMismatch {
            op_span: operator.span,
            lhs_ty: left.get_type(),
            lhs_span: left.span()?,
            rhs_ty: right.get_type(),
            rhs_span: right.span()?,
        });
    }

    let lhs_span = left.span()?;
    let lhs = lhs.as_series(lhs_span)?;

    match operator.item {
        Operator::Plus => match &right {
            Value::Int { val, .. } => {
                compute_series_i64(&lhs, *val, <ChunkedArray<Int64Type>>::add, lhs_span)
            }
            Value::Float { val, .. } => {
                compute_series_decimal(&lhs, *val, <ChunkedArray<Float64Type>>::add, lhs_span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: left.get_type(),
                lhs_span: left.span()?,
                rhs_ty: right.get_type(),
                rhs_span: right.span()?,
            }),
        },
        Operator::Minus => match &right {
            Value::Int { val, .. } => {
                compute_series_i64(&lhs, *val, <ChunkedArray<Int64Type>>::sub, lhs_span)
            }
            Value::Float { val, .. } => {
                compute_series_decimal(&lhs, *val, <ChunkedArray<Float64Type>>::sub, lhs_span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: left.get_type(),
                lhs_span: left.span()?,
                rhs_ty: right.get_type(),
                rhs_span: right.span()?,
            }),
        },
        Operator::Multiply => match &right {
            Value::Int { val, .. } => {
                compute_series_i64(&lhs, *val, <ChunkedArray<Int64Type>>::mul, lhs_span)
            }
            Value::Float { val, .. } => {
                compute_series_decimal(&lhs, *val, <ChunkedArray<Float64Type>>::mul, lhs_span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: left.get_type(),
                lhs_span: left.span()?,
                rhs_ty: right.get_type(),
                rhs_span: right.span()?,
            }),
        },
        Operator::Divide => match &right {
            Value::Int { val, span } => {
                if *val == 0 {
                    Err(ShellError::DivisionByZero(*span))
                } else {
                    compute_series_i64(&lhs, *val, <ChunkedArray<Int64Type>>::div, lhs_span)
                }
            }
            Value::Float { val, span } => {
                if val.is_zero() {
                    Err(ShellError::DivisionByZero(*span))
                } else {
                    compute_series_decimal(&lhs, *val, <ChunkedArray<Float64Type>>::div, lhs_span)
                }
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: left.get_type(),
                lhs_span: left.span()?,
                rhs_ty: right.get_type(),
                rhs_span: right.span()?,
            }),
        },
        Operator::Equal => match &right {
            Value::Int { val, .. } => compare_series_i64(&lhs, *val, ChunkedArray::equal, lhs_span),
            Value::Float { val, .. } => {
                compare_series_decimal(&lhs, *val, ChunkedArray::equal, lhs_span)
            }
            Value::String { val, .. } => {
                let equal_pattern = format!("^{}$", val);
                contains_series_pat(&lhs, &equal_pattern, lhs_span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: left.get_type(),
                lhs_span: left.span()?,
                rhs_ty: right.get_type(),
                rhs_span: right.span()?,
            }),
        },
        Operator::NotEqual => match &right {
            Value::Int { val, .. } => {
                compare_series_i64(&lhs, *val, ChunkedArray::not_equal, lhs_span)
            }
            Value::Float { val, .. } => {
                compare_series_decimal(&lhs, *val, ChunkedArray::not_equal, lhs_span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: left.get_type(),
                lhs_span: left.span()?,
                rhs_ty: right.get_type(),
                rhs_span: right.span()?,
            }),
        },
        Operator::LessThan => match &right {
            Value::Int { val, .. } => compare_series_i64(&lhs, *val, ChunkedArray::lt, lhs_span),
            Value::Float { val, .. } => {
                compare_series_decimal(&lhs, *val, ChunkedArray::lt, lhs_span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: left.get_type(),
                lhs_span: left.span()?,
                rhs_ty: right.get_type(),
                rhs_span: right.span()?,
            }),
        },
        Operator::LessThanOrEqual => match &right {
            Value::Int { val, .. } => compare_series_i64(&lhs, *val, ChunkedArray::lt_eq, lhs_span),
            Value::Float { val, .. } => {
                compare_series_decimal(&lhs, *val, ChunkedArray::lt_eq, lhs_span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: left.get_type(),
                lhs_span: left.span()?,
                rhs_ty: right.get_type(),
                rhs_span: right.span()?,
            }),
        },
        Operator::GreaterThan => match &right {
            Value::Int { val, .. } => compare_series_i64(&lhs, *val, ChunkedArray::gt, lhs_span),
            Value::Float { val, .. } => {
                compare_series_decimal(&lhs, *val, ChunkedArray::gt, lhs_span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: left.get_type(),
                lhs_span: left.span()?,
                rhs_ty: right.get_type(),
                rhs_span: right.span()?,
            }),
        },
        Operator::GreaterThanOrEqual => match &right {
            Value::Int { val, .. } => compare_series_i64(&lhs, *val, ChunkedArray::gt_eq, lhs_span),
            Value::Float { val, .. } => {
                compare_series_decimal(&lhs, *val, ChunkedArray::gt_eq, lhs_span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: left.get_type(),
                lhs_span: left.span()?,
                rhs_ty: right.get_type(),
                rhs_span: right.span()?,
            }),
        },
        Operator::Contains => match &right {
            Value::String { val, .. } => contains_series_pat(&lhs, val, lhs_span),
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: left.get_type(),
                lhs_span: left.span()?,
                rhs_ty: right.get_type(),
                rhs_span: right.span()?,
            }),
        },
        _ => Err(ShellError::OperatorMismatch {
            op_span: operator.span,
            lhs_ty: left.get_type(),
            lhs_span: left.span()?,
            rhs_ty: right.get_type(),
            rhs_span: right.span()?,
        }),
    }
}

fn compute_series_i64<F>(series: &Series, val: i64, f: F, span: Span) -> Result<Value, ShellError>
where
    F: Fn(ChunkedArray<Int64Type>, i64) -> ChunkedArray<Int64Type>,
{
    match series.dtype() {
        DataType::UInt32 | DataType::Int32 | DataType::UInt64 => {
            let to_i64 = series.cast(&DataType::Int64);

            match to_i64 {
                Ok(series) => {
                    let casted = series.i64();
                    compute_casted_i64(casted, val, f, span)
                }
                Err(e) => Err(ShellError::SpannedLabeledError(
                    "Unable to cast to i64".into(),
                    e.to_string(),
                    span,
                )),
            }
        }
        DataType::Int64 => {
            let casted = series.i64();
            compute_casted_i64(casted, val, f, span)
        }
        _ => Err(ShellError::SpannedLabeledError(
            "Incorrect type".into(),
            format!(
                "Series of type {} can not be used for operations with an i64 value",
                series.dtype()
            ),
            span,
        )),
    }
}

fn compute_casted_i64<F>(
    casted: Result<&ChunkedArray<Int64Type>, PolarsError>,
    val: i64,
    f: F,
    span: Span,
) -> Result<Value, ShellError>
where
    F: Fn(ChunkedArray<Int64Type>, i64) -> ChunkedArray<Int64Type>,
{
    match casted {
        Ok(casted) => {
            let res = f(casted.clone(), val);
            let res = res.into_series();
            NuDataFrame::series_to_value(res, span)
        }
        Err(e) => Err(ShellError::SpannedLabeledError(
            "Unable to cast to i64".into(),
            e.to_string(),
            span,
        )),
    }
}

fn compute_series_decimal<F>(
    series: &Series,
    val: f64,
    f: F,
    span: Span,
) -> Result<Value, ShellError>
where
    F: Fn(ChunkedArray<Float64Type>, f64) -> ChunkedArray<Float64Type>,
{
    match series.dtype() {
        DataType::Float32 => {
            let to_f64 = series.cast(&DataType::Float64);

            match to_f64 {
                Ok(series) => {
                    let casted = series.f64();
                    compute_casted_f64(casted, val, f, span)
                }
                Err(e) => Err(ShellError::SpannedLabeledError(
                    "Unable to cast to f64".into(),
                    e.to_string(),
                    span,
                )),
            }
        }
        DataType::Float64 => {
            let casted = series.f64();
            compute_casted_f64(casted, val, f, span)
        }
        _ => Err(ShellError::SpannedLabeledError(
            "Incorrect type".into(),
            format!(
                "Series of type {} can not be used for operations with a decimal value",
                series.dtype()
            ),
            span,
        )),
    }
}

fn compute_casted_f64<F>(
    casted: Result<&ChunkedArray<Float64Type>, PolarsError>,
    val: f64,
    f: F,
    span: Span,
) -> Result<Value, ShellError>
where
    F: Fn(ChunkedArray<Float64Type>, f64) -> ChunkedArray<Float64Type>,
{
    match casted {
        Ok(casted) => {
            let res = f(casted.clone(), val);
            let res = res.into_series();
            NuDataFrame::series_to_value(res, span)
        }
        Err(e) => Err(ShellError::SpannedLabeledError(
            "Unable to cast to f64".into(),
            e.to_string(),
            span,
        )),
    }
}

fn compare_series_i64<F>(series: &Series, val: i64, f: F, span: Span) -> Result<Value, ShellError>
where
    F: Fn(&ChunkedArray<Int64Type>, i64) -> ChunkedArray<BooleanType>,
{
    match series.dtype() {
        DataType::UInt32 | DataType::Int32 | DataType::UInt64 => {
            let to_i64 = series.cast(&DataType::Int64);

            match to_i64 {
                Ok(series) => {
                    let casted = series.i64();
                    compare_casted_i64(casted, val, f, span)
                }
                Err(e) => Err(ShellError::SpannedLabeledError(
                    "Unable to cast to f64".into(),
                    e.to_string(),
                    span,
                )),
            }
        }
        DataType::Int64 => {
            let casted = series.i64();
            compare_casted_i64(casted, val, f, span)
        }
        _ => Err(ShellError::SpannedLabeledError(
            "Incorrect type".into(),
            format!(
                "Series of type {} can not be used for operations with an i64 value",
                series.dtype()
            ),
            span,
        )),
    }
}

fn compare_casted_i64<F>(
    casted: Result<&ChunkedArray<Int64Type>, PolarsError>,
    val: i64,
    f: F,
    span: Span,
) -> Result<Value, ShellError>
where
    F: Fn(&ChunkedArray<Int64Type>, i64) -> ChunkedArray<BooleanType>,
{
    match casted {
        Ok(casted) => {
            let res = f(casted, val);
            let res = res.into_series();
            NuDataFrame::series_to_value(res, span)
        }
        Err(e) => Err(ShellError::SpannedLabeledError(
            "Unable to cast to i64".into(),
            e.to_string(),
            span,
        )),
    }
}

fn compare_series_decimal<F>(
    series: &Series,
    val: f64,
    f: F,
    span: Span,
) -> Result<Value, ShellError>
where
    F: Fn(&ChunkedArray<Float64Type>, f64) -> ChunkedArray<BooleanType>,
{
    match series.dtype() {
        DataType::Float32 => {
            let to_f64 = series.cast(&DataType::Float64);

            match to_f64 {
                Ok(series) => {
                    let casted = series.f64();
                    compare_casted_f64(casted, val, f, span)
                }
                Err(e) => Err(ShellError::SpannedLabeledError(
                    "Unable to cast to i64".into(),
                    e.to_string(),
                    span,
                )),
            }
        }
        DataType::Float64 => {
            let casted = series.f64();
            compare_casted_f64(casted, val, f, span)
        }
        _ => Err(ShellError::SpannedLabeledError(
            "Incorrect type".into(),
            format!(
                "Series of type {} can not be used for operations with a decimal value",
                series.dtype()
            ),
            span,
        )),
    }
}

fn compare_casted_f64<F>(
    casted: Result<&ChunkedArray<Float64Type>, PolarsError>,
    val: f64,
    f: F,
    span: Span,
) -> Result<Value, ShellError>
where
    F: Fn(&ChunkedArray<Float64Type>, f64) -> ChunkedArray<BooleanType>,
{
    match casted {
        Ok(casted) => {
            let res = f(casted, val);
            let res = res.into_series();
            NuDataFrame::series_to_value(res, span)
        }
        Err(e) => Err(ShellError::SpannedLabeledError(
            "Unable to cast to f64".into(),
            e.to_string(),
            span,
        )),
    }
}

fn contains_series_pat(series: &Series, pat: &str, span: Span) -> Result<Value, ShellError> {
    let casted = series.utf8();
    match casted {
        Ok(casted) => {
            let res = casted.contains(pat);

            match res {
                Ok(res) => {
                    let res = res.into_series();
                    NuDataFrame::series_to_value(res, span)
                }
                Err(e) => Err(ShellError::SpannedLabeledError(
                    "Error using contains".into(),
                    e.to_string(),
                    span,
                )),
            }
        }
        Err(e) => Err(ShellError::SpannedLabeledError(
            "Unable to cast to string".into(),
            e.to_string(),
            span,
        )),
    }
}
