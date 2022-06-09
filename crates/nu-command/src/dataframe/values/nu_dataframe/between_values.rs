use super::{operations::Axis, NuDataFrame};

use nu_protocol::{ast::Operator, ShellError, Span, Spanned, Type, Value};
use num::Zero;
use polars::prelude::{
    BooleanType, ChunkCompare, ChunkedArray, DataType, Float64Type, Int64Type, IntoSeries,
    NumOpsDispatchChecked, PolarsError, Series, TimeUnit,
};
use std::ops::{Add, BitAnd, BitOr, Div, Mul, Sub};

pub(super) fn between_dataframes(
    operator: Spanned<Operator>,
    lhs: &NuDataFrame,
    rhs: (&NuDataFrame, Type),
    span: Span,
) -> Result<Value, ShellError> {
    match operator.item {
        Operator::Plus => match lhs.append_df(rhs.0, Axis::Row, span) {
            Ok(df) => Ok(df.into_value()),
            Err(e) => Err(e),
        },
        _ => Err(ShellError::OperatorMismatch {
            op_span: operator.span,
            lhs_ty: Type::Custom,
            rhs_ty: rhs.1,
        }),
    }
}

pub(super) fn compute_between_series(
    operator: Spanned<Operator>,
    lhs: &Series,
    rhs: (&Series, Type),
    span: Span,
) -> Result<Value, ShellError> {
    match operator.item {
        Operator::Plus => {
            let mut res = lhs.0 + rhs.0;
            let name = format!("sum_{}_{}", lhs.0.name(), rhs.0.name());
            res.rename(&name);
            NuDataFrame::series_to_value(res, span)
        }
        Operator::Minus => {
            let mut res = lhs.0 - rhs.0;
            let name = format!("sub_{}_{}", lhs.0.name(), rhs.0.name());
            res.rename(&name);
            NuDataFrame::series_to_value(res, span)
        }
        Operator::Multiply => {
            let mut res = lhs.0 * rhs.0;
            let name = format!("mul_{}_{}", lhs.0.name(), rhs.0.name());
            res.rename(&name);
            NuDataFrame::series_to_value(res, span)
        }
        Operator::Divide => {
            let res = lhs.0.checked_div(rhs.0);
            match res {
                Ok(mut res) => {
                    let name = format!("div_{}_{}", lhs.0.name(), rhs.0.name());
                    res.rename(&name);
                    NuDataFrame::series_to_value(res, span)
                }
                Err(e) => Err(ShellError::GenericError(
                    "Division error".into(),
                    e.to_string(),
                    Some(operator.span),
                    None,
                    Vec::new(),
                )),
            }
        }
        Operator::Equal => {
            let name = format!("eq_{}_{}", lhs.0.name(), rhs.0.name());
            let res = compare_series(
                lhs.0,
                rhs.0,
                name.as_str(),
                Some(operator.span),
                Series::equal,
            )?;
            NuDataFrame::series_to_value(res, span)
        }
        Operator::NotEqual => {
            let name = format!("neq_{}_{}", lhs.0.name(), rhs.0.name());
            let res = compare_series(
                lhs.0,
                rhs.0,
                name.as_str(),
                Some(operator.span),
                Series::equal,
            )?;
            NuDataFrame::series_to_value(res, span)
        }
        Operator::LessThan => {
            let name = format!("lt_{}_{}", lhs.0.name(), rhs.0.name());
            let res = compare_series(
                lhs.0,
                rhs.0,
                name.as_str(),
                Some(operator.span),
                Series::equal,
            )?;
            NuDataFrame::series_to_value(res, span)
        }
        Operator::LessThanOrEqual => {
            let name = format!("lte_{}_{}", lhs.0.name(), rhs.0.name());
            let res = compare_series(
                lhs.0,
                rhs.0,
                name.as_str(),
                Some(operator.span),
                Series::equal,
            )?;
            NuDataFrame::series_to_value(res, span)
        }
        Operator::GreaterThan => {
            let name = format!("gt_{}_{}", lhs.0.name(), rhs.0.name());
            let res = compare_series(
                lhs.0,
                rhs.0,
                name.as_str(),
                Some(operator.span),
                Series::equal,
            )?;
            NuDataFrame::series_to_value(res, span)
        }
        Operator::GreaterThanOrEqual => {
            let name = format!("gte_{}_{}", lhs.0.name(), rhs.0.name());
            let res = compare_series(
                lhs.0,
                rhs.0,
                name.as_str(),
                Some(operator.span),
                Series::equal,
            )?;
            NuDataFrame::series_to_value(res, span)
        }
        Operator::And => match lhs.0.dtype() {
            DataType::Boolean => {
                let lhs_cast = lhs.0.bool();
                let rhs_cast = rhs.0.bool();

                match (lhs_cast, rhs_cast) {
                    (Ok(l), Ok(r)) => {
                        let mut res = l.bitand(r).into_series();
                        let name = format!("and_{}_{}", lhs.0.name(), rhs.0.name());
                        res.rename(&name);
                        NuDataFrame::series_to_value(res, span)
                    }
                    _ => Err(ShellError::GenericError(
                        "Incompatible types".into(),
                        "unable to cast to boolean".into(),
                        Some(operator.span),
                        None,
                        Vec::new(),
                    )),
                }
            }
            _ => Err(ShellError::IncompatibleParametersSingle(
                format!(
                    "Operation {} can only be done with boolean values",
                    operator.item
                ),
                span,
            )),
        },
        Operator::Or => match lhs.0.dtype() {
            DataType::Boolean => {
                let lhs_cast = lhs.0.bool();
                let rhs_cast = rhs.0.bool();

                match (lhs_cast, rhs_cast) {
                    (Ok(l), Ok(r)) => {
                        let mut res = l.bitor(r).into_series();
                        let name = format!("or_{}_{}", lhs.0.name(), rhs.0.name());
                        res.rename(&name);
                        NuDataFrame::series_to_value(res, span)
                    }
                    _ => Err(ShellError::GenericError(
                        "Incompatible types".into(),
                        "unable to cast to boolean".into(),
                        Some(operator.span),
                        None,
                        Vec::new(),
                    )),
                }
            }
            _ => Err(ShellError::IncompatibleParametersSingle(
                format!(
                    "Operation {} can only be done with boolean values",
                    operator.item
                ),
                span,
            )),
        },
        _ => Err(ShellError::OperatorMismatch {
            op_span: operator.span,
            lhs_ty: Type::Custom,
            rhs_ty: rhs.1,
        }),
    }
}

fn compare_series<'s, F>(
    lhs: &'s Series,
    rhs: &'s Series,
    name: &'s str,
    span: Option<Span>,
    f: F,
) -> Result<Series, ShellError>
where
    F: Fn(&'s Series, &'s Series) -> Result<ChunkedArray<BooleanType>, PolarsError>,
{
    let mut res = f(lhs, rhs)
        .map_err(|e| {
            ShellError::GenericError(
                "Equality error".into(),
                e.to_string(),
                span,
                None,
                Vec::new(),
            )
        })?
        .into_series();

    res.rename(name);
    Ok(res)
}

pub(super) fn compute_series_single_value(
    operator: Spanned<Operator>,
    lhs: &NuDataFrame,
    right: &Value,
    span: Span,
) -> Result<Value, ShellError> {
    if !lhs.0.is_series() {
        return Err(ShellError::OperatorMismatch {
            op_span: operator.span,
            lhs_ty: lhs.1,
            rhs_ty: right.get_type(),
        });
    }

    let lhs = lhs.0.as_series(span)?;

    match operator.item {
        Operator::Plus => match &right {
            Value::Int(val) => {
                compute_series_i64(&lhs, *val, <ChunkedArray<Int64Type>>::add, Some(span))
            }
            Value::Float(val) => {
                compute_series_decimal(&lhs, *val, <ChunkedArray<Float64Type>>::add, Some(span))
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: lhs.1,
                rhs_ty: right.get_type(),
            }),
        },
        Operator::Minus => match &right {
            Value::Int(val) => {
                compute_series_i64(&lhs, *val, <ChunkedArray<Int64Type>>::sub, Some(span))
            }
            Value::Float(val) => {
                compute_series_decimal(&lhs, *val, <ChunkedArray<Float64Type>>::sub, Some(span))
            }
        Operator::Multiply => match &right {
            Value::Int(val) => compute_series_i64(&lhs, *val, <ChunkedArray<Int64Type>>::mul, span),
            Value::Float(val) => {
                compute_series_decimal(&lhs, *val, <ChunkedArray<Float64Type>>::mul, span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: lhs.1,
                rhs_ty: right.get_type(),
            }),
        },
        Operator::Divide => match &right {
            Value::Int(val) => {
                if *val == 0 {
                    Err(ShellError::DivisionByZero(*span))
                } else {
                    compute_series_i64(&lhs, *val, <ChunkedArray<Int64Type>>::div, span)
                }
            }
            Value::Float(val) => {
                if val.is_zero() {
                    Err(ShellError::DivisionByZero(*span))
                } else {
                    compute_series_decimal(&lhs, *val, <ChunkedArray<Float64Type>>::div, span)
                }
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: lhs.1,
                rhs_ty: right.get_type(),
            }),
        },
        Operator::Equal => match &right {
            Value::Int(val) => compare_series_i64(&lhs, *val, ChunkedArray::equal, span),
            Value::Float(val) => compare_series_decimal(&lhs, *val, ChunkedArray::equal, span),
            Value::String(val) => {
                let equal_pattern = format!("^{}$", regex::escape(val));
                contains_series_pat(&lhs, &equal_pattern, span)
            }
            Value::Date(val) => {
                compare_series_i64(&lhs, val.timestamp_millis(), ChunkedArray::equal, span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: lhs.1,
                rhs_ty: right.get_type(),
            }),
        },
        Operator::NotEqual => match &right {
            Value::Int(val) => compare_series_i64(&lhs, *val, ChunkedArray::not_equal, span),
            Value::Float(val) => compare_series_decimal(&lhs, *val, ChunkedArray::not_equal, span),
            Value::Date(val) => {
                compare_series_i64(&lhs, val.timestamp_millis(), ChunkedArray::not_equal, span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: lhs.1,
                rhs_ty: right.get_type(),
            }),
        },
        Operator::LessThan => match &right {
            Value::Int(val) => compare_series_i64(&lhs, *val, ChunkedArray::lt, span),
            Value::Float(val) => compare_series_decimal(&lhs, *val, ChunkedArray::lt, span),
            Value::Date(val) => {
                compare_series_i64(&lhs, val.timestamp_millis(), ChunkedArray::lt, span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: lhs.1,
                rhs_ty: right.get_type(),
            }),
        },
        Operator::LessThanOrEqual => match &right {
            Value::Int(val) => compare_series_i64(&lhs, *val, ChunkedArray::lt_eq, span),
            Value::Float(val) => compare_series_decimal(&lhs, *val, ChunkedArray::lt_eq, span),
            Value::Date(val) => {
                compare_series_i64(&lhs, val.timestamp_millis(), ChunkedArray::lt_eq, span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: lhs.1,
                rhs_ty: right.get_type(),
            }),
        },
        Operator::GreaterThan => match &right {
            Value::Int(val) => compare_series_i64(&lhs, *val, ChunkedArray::gt, span),
            Value::Float(val) => compare_series_decimal(&lhs, *val, ChunkedArray::gt, span),
            Value::Date(val) => {
                compare_series_i64(&lhs, val.timestamp_millis(), ChunkedArray::gt, span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: lhs.1,
                rhs_ty: right.get_type(),
            }),
        },
        Operator::GreaterThanOrEqual => match &right {
            Value::Int(val) => compare_series_i64(&lhs, *val, ChunkedArray::gt_eq, span),
            Value::Float(val) => compare_series_decimal(&lhs, *val, ChunkedArray::gt_eq, span),
            Value::Date(val) => {
                compare_series_i64(&lhs, val.timestamp_millis(), ChunkedArray::gt_eq, span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: lhs.1,
                rhs_ty: right.get_type(),
            }),
        },
        // TODO: update this to do a regex match instead of a simple contains?
        Operator::RegexMatch => match &right {
            Value::String(val) => contains_series_pat(&lhs, val, span),
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: lhs.1,
                rhs_ty: right.get_type(),
            }),
        },
        Operator::StartsWith => match &right {
            Value::String(val) => {
                let starts_with_pattern = format!("^{}", regex::escape(val));
                contains_series_pat(&lhs, &starts_with_pattern, span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: lhs.1,
                rhs_ty: right.get_type(),
            }),
        },
        Operator::EndsWith => match &right {
            Value::String(val) => {
                let ends_with_pattern = format!("{}$", regex::escape(val));
                contains_series_pat(&lhs, &ends_with_pattern, span)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: operator.span,
                lhs_ty: lhs.1,
                rhs_ty: right.get_type(),
            }),
        },
        _ => Err(ShellError::OperatorMismatch {
            op_span: operator.span,
            lhs_ty: Type::Custom,
            rhs_ty: right.get_type(),
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
                Err(e) => Err(ShellError::GenericError(
                    "Unable to cast to i64".into(),
                    e.to_string(),
                    Some(span),
                    None,
                    Vec::new(),
                )),
            }
        }
        DataType::Int64 => {
            let casted = series.i64();
            compute_casted_i64(casted, val, f, span)
        }
        _ => Err(ShellError::GenericError(
            "Incorrect type".into(),
            format!(
                "Series of type {} can not be used for operations with an i64 value",
                series.dtype()
            ),
            Some(span),
            None,
            Vec::new(),
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
        Err(e) => Err(ShellError::GenericError(
            "Unable to cast to i64".into(),
            e.to_string(),
            Some(span),
            None,
            Vec::new(),
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
                Err(e) => Err(ShellError::GenericError(
                    "Unable to cast to f64".into(),
                    e.to_string(),
                    Some(span),
                    None,
                    Vec::new(),
                )),
            }
        }
        DataType::Float64 => {
            let casted = series.f64();
            compute_casted_f64(casted, val, f, span)
        }
        _ => Err(ShellError::GenericError(
            "Incorrect type".into(),
            format!(
                "Series of type {} can not be used for operations with a decimal value",
                series.dtype()
            ),
            Some(span),
            None,
            Vec::new(),
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
        Err(e) => Err(ShellError::GenericError(
            "Unable to cast to f64".into(),
            e.to_string(),
            Some(span),
            None,
            Vec::new(),
        )),
    }
}

fn compare_series_i64<F>(series: &Series, val: i64, f: F, span: Span) -> Result<Value, ShellError>
where
    F: Fn(&ChunkedArray<Int64Type>, i64) -> ChunkedArray<BooleanType>,
{
    match series.dtype() {
        DataType::UInt32
        | DataType::Int32
        | DataType::UInt64
        | DataType::Datetime(TimeUnit::Milliseconds, _) => {
            let to_i64 = series.cast(&DataType::Int64);

            match to_i64 {
                Ok(series) => {
                    let casted = series.i64();
                    compare_casted_i64(casted, val, f, span)
                }
                Err(e) => Err(ShellError::GenericError(
                    "Unable to cast to f64".into(),
                    e.to_string(),
                    Some(span),
                    None,
                    Vec::new(),
                )),
            }
        }
        DataType::Date => {
            let to_i64 = series.cast(&DataType::Int64);

            match to_i64 {
                Ok(series) => {
                    let nanosecs_per_day: i64 = 24 * 60 * 60 * 1_000_000_000;
                    let casted = series
                        .i64()
                        .map(|chunked| chunked.mul(nanosecs_per_day))
                        .expect("already checked for casting");
                    compare_casted_i64(Ok(&casted), val, f, span)
                }
                Err(e) => Err(ShellError::GenericError(
                    "Unable to cast to f64".into(),
                    e.to_string(),
                    Some(span),
                    None,
                    Vec::new(),
                )),
            }
        }
        DataType::Int64 => {
            let casted = series.i64();
            compare_casted_i64(casted, val, f, span)
        }
        _ => Err(ShellError::GenericError(
            "Incorrect type".into(),
            format!(
                "Series of type {} can not be used for operations with an i64 value",
                series.dtype()
            ),
            Some(span),
            None,
            Vec::new(),
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
        Err(e) => Err(ShellError::GenericError(
            "Unable to cast to i64".into(),
            e.to_string(),
            Some(span),
            None,
            Vec::new(),
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
                Err(e) => Err(ShellError::GenericError(
                    "Unable to cast to i64".into(),
                    e.to_string(),
                    Some(span),
                    None,
                    Vec::new(),
                )),
            }
        }
        DataType::Float64 => {
            let casted = series.f64();
            compare_casted_f64(casted, val, f, span)
        }
        _ => Err(ShellError::GenericError(
            "Incorrect type".into(),
            format!(
                "Series of type {} can not be used for operations with a decimal value",
                series.dtype()
            ),
            Some(span),
            None,
            Vec::new(),
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
        Err(e) => Err(ShellError::GenericError(
            "Unable to cast to f64".into(),
            e.to_string(),
            Some(span),
            None,
            Vec::new(),
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
                Err(e) => Err(ShellError::GenericError(
                    "Error using contains".into(),
                    e.to_string(),
                    Some(span),
                    None,
                    Vec::new(),
                )),
            }
        }
        Err(e) => Err(ShellError::GenericError(
            "Unable to cast to string".into(),
            e.to_string(),
            Some(span),
            None,
            Vec::new(),
        )),
    }
}
