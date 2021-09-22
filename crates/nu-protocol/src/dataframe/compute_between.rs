use bigdecimal::BigDecimal;
use nu_errors::ShellError;
use nu_source::Span;
use num_traits::ToPrimitive;

use super::{Axis, NuDataFrame};
use crate::hir::Operator;
use crate::{Primitive, ShellTypeName, UntaggedValue, Value};

use polars::prelude::{
    BooleanType, ChunkCompare, ChunkedArray, DataType, Float64Type, Int64Type, IntoSeries,
    NumOpsDispatchChecked, PolarsError, Series,
};
use std::ops::{Add, BitAnd, BitOr, Div, Mul, Sub};

pub fn compute_between_dataframes(
    operator: Operator,
    left: &Value,
    right: &Value,
) -> Result<UntaggedValue, (&'static str, &'static str)> {
    if let (UntaggedValue::DataFrame(lhs), UntaggedValue::DataFrame(rhs)) =
        (&left.value, &right.value)
    {
        let operation_span = right.tag.span.merge(left.tag.span);
        match (lhs.is_series(), rhs.is_series()) {
            (true, true) => {
                let lhs = &lhs
                    .as_series(&left.tag.span)
                    .expect("Already checked that is a series");
                let rhs = &rhs
                    .as_series(&right.tag.span)
                    .expect("Already checked that is a series");

                if lhs.dtype() != rhs.dtype() {
                    return Ok(UntaggedValue::Error(
                        ShellError::labeled_error_with_secondary(
                            "Mixed datatypes",
                            "this datatype does not match the right hand side datatype",
                            &left.tag.span,
                            format!(
                                "Perhaps you want to change this datatype to '{}'",
                                lhs.as_ref().dtype()
                            ),
                            &right.tag.span,
                        ),
                    ));
                }

                if lhs.len() != rhs.len() {
                    return Ok(UntaggedValue::Error(ShellError::labeled_error(
                        "Different length",
                        "this column length does not match the right hand column length",
                        &left.tag.span,
                    )));
                }

                compute_between_series(operator, lhs, rhs, &operation_span)
            }
            _ => {
                if lhs.as_ref().height() != rhs.as_ref().height() {
                    return Ok(UntaggedValue::Error(
                        ShellError::labeled_error_with_secondary(
                            "Mixed datatypes",
                            "this datatype size does not match the right hand side datatype",
                            &left.tag.span,
                            "Perhaps you want to select another dataframe with same number of rows",
                            &right.tag.span,
                        ),
                    ));
                }

                between_dataframes(operator, lhs, rhs, &operation_span)
            }
        }
    } else {
        Err((left.type_name(), right.type_name()))
    }
}

pub fn between_dataframes(
    operator: Operator,
    lhs: &NuDataFrame,
    rhs: &NuDataFrame,
    operation_span: &Span,
) -> Result<UntaggedValue, (&'static str, &'static str)> {
    match operator {
        Operator::Plus => match lhs.append_df(rhs, Axis::Row, operation_span) {
            Ok(df) => Ok(df.into_untagged()),
            Err(e) => Ok(UntaggedValue::Error(ShellError::labeled_error(
                "Appending error",
                e.to_string(),
                operation_span,
            ))),
        },
        _ => Ok(UntaggedValue::Error(ShellError::labeled_error(
            "Incorrect datatype",
            "unable to use this datatype for this operation",
            operation_span,
        ))),
    }
}

pub fn compute_between_series(
    operator: Operator,
    lhs: &Series,
    rhs: &Series,
    operation_span: &Span,
) -> Result<UntaggedValue, (&'static str, &'static str)> {
    match operator {
        Operator::Plus => {
            let mut res = lhs + rhs;
            let name = format!("sum_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            Ok(NuDataFrame::series_to_untagged(res, operation_span))
        }
        Operator::Minus => {
            let mut res = lhs - rhs;
            let name = format!("sub_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            Ok(NuDataFrame::series_to_untagged(res, operation_span))
        }
        Operator::Multiply => {
            let mut res = lhs * rhs;
            let name = format!("mul_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            Ok(NuDataFrame::series_to_untagged(res, operation_span))
        }
        Operator::Divide => {
            let res = lhs.checked_div(rhs);
            match res {
                Ok(mut res) => {
                    let name = format!("div_{}_{}", lhs.name(), rhs.name());
                    res.rename(&name);
                    Ok(NuDataFrame::series_to_untagged(res, operation_span))
                }
                Err(e) => Ok(UntaggedValue::Error(ShellError::labeled_error(
                    "Division error",
                    e.to_string(),
                    operation_span,
                ))),
            }
        }
        Operator::Equal => {
            let mut res = Series::eq(lhs, rhs).into_series();
            let name = format!("eq_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            Ok(NuDataFrame::series_to_untagged(res, operation_span))
        }
        Operator::NotEqual => {
            let mut res = Series::neq(lhs, rhs).into_series();
            let name = format!("neq_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            Ok(NuDataFrame::series_to_untagged(res, operation_span))
        }
        Operator::LessThan => {
            let mut res = Series::lt(lhs, rhs).into_series();
            let name = format!("lt_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            Ok(NuDataFrame::series_to_untagged(res, operation_span))
        }
        Operator::LessThanOrEqual => {
            let mut res = Series::lt_eq(lhs, rhs).into_series();
            let name = format!("lte_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            Ok(NuDataFrame::series_to_untagged(res, operation_span))
        }
        Operator::GreaterThan => {
            let mut res = Series::gt(lhs, rhs).into_series();
            let name = format!("gt_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            Ok(NuDataFrame::series_to_untagged(res, operation_span))
        }
        Operator::GreaterThanOrEqual => {
            let mut res = Series::gt_eq(lhs, rhs).into_series();
            let name = format!("gte_{}_{}", lhs.name(), rhs.name());
            res.rename(&name);
            Ok(NuDataFrame::series_to_untagged(res, operation_span))
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
                        Ok(NuDataFrame::series_to_untagged(res, operation_span))
                    }
                    _ => Ok(UntaggedValue::Error(ShellError::labeled_error(
                        "Casting error",
                        "unable to cast to boolean",
                        operation_span,
                    ))),
                }
            }
            _ => Ok(UntaggedValue::Error(ShellError::labeled_error(
                "Incorrect datatype",
                "And operation can only be done with boolean values",
                operation_span,
            ))),
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
                        Ok(NuDataFrame::series_to_untagged(res, operation_span))
                    }
                    _ => Ok(UntaggedValue::Error(ShellError::labeled_error(
                        "Casting error",
                        "unable to cast to boolean",
                        operation_span,
                    ))),
                }
            }
            _ => Ok(UntaggedValue::Error(ShellError::labeled_error(
                "Incorrect datatype",
                "And operation can only be done with boolean values",
                operation_span,
            ))),
        },
        _ => Ok(UntaggedValue::Error(ShellError::labeled_error(
            "Incorrect datatype",
            "unable to use this datatype for this operation",
            operation_span,
        ))),
    }
}

pub fn compute_series_single_value(
    operator: Operator,
    left: &Value,
    right: &Value,
) -> Result<UntaggedValue, (&'static str, &'static str)> {
    if let (UntaggedValue::DataFrame(lhs), UntaggedValue::Primitive(_)) =
        (&left.value, &right.value)
    {
        let lhs = match lhs.as_series(&left.tag.span) {
            Ok(series) => series,
            Err(e) => return Ok(UntaggedValue::Error(e)),
        };

        match operator {
            Operator::Plus => match &right.value {
                UntaggedValue::Primitive(Primitive::Int(val)) => Ok(compute_series_i64(
                    &lhs,
                    val,
                    <ChunkedArray<Int64Type>>::add,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compute_series_i64(
                    &lhs,
                    &val.to_i64()
                        .expect("Internal error: protocol did not use compatible decimal"),
                    <ChunkedArray<Int64Type>>::add,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(compute_series_decimal(
                    &lhs,
                    val,
                    <ChunkedArray<Float64Type>>::add,
                    &left.tag.span,
                )),
                _ => Ok(UntaggedValue::Error(
                    ShellError::labeled_error_with_secondary(
                        "Operation unavailable",
                        "unable to sum this value to the series",
                        &right.tag.span,
                        "Only int, bigInt or decimal values are allowed",
                        &right.tag.span,
                    ),
                )),
            },
            Operator::Minus => match &right.value {
                UntaggedValue::Primitive(Primitive::Int(val)) => Ok(compute_series_i64(
                    &lhs,
                    val,
                    <ChunkedArray<Int64Type>>::sub,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compute_series_i64(
                    &lhs,
                    &val.to_i64()
                        .expect("Internal error: protocol did not use compatible decimal"),
                    <ChunkedArray<Int64Type>>::sub,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(compute_series_decimal(
                    &lhs,
                    val,
                    <ChunkedArray<Float64Type>>::sub,
                    &left.tag.span,
                )),
                _ => Ok(UntaggedValue::Error(
                    ShellError::labeled_error_with_secondary(
                        "Operation unavailable",
                        "unable to subtract this value to the series",
                        &right.tag.span,
                        "Only int, bigInt or decimal values are allowed",
                        &right.tag.span,
                    ),
                )),
            },
            Operator::Multiply => match &right.value {
                UntaggedValue::Primitive(Primitive::Int(val)) => Ok(compute_series_i64(
                    &lhs,
                    val,
                    <ChunkedArray<Int64Type>>::mul,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compute_series_i64(
                    &lhs,
                    &val.to_i64()
                        .expect("Internal error: protocol did not use compatible decimal"),
                    <ChunkedArray<Int64Type>>::mul,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(compute_series_decimal(
                    &lhs,
                    val,
                    <ChunkedArray<Float64Type>>::mul,
                    &left.tag.span,
                )),
                _ => Ok(UntaggedValue::Error(
                    ShellError::labeled_error_with_secondary(
                        "Operation unavailable",
                        "unable to multiply this value to the series",
                        &right.tag.span,
                        "Only int, bigInt or decimal values are allowed",
                        &right.tag.span,
                    ),
                )),
            },
            Operator::Divide => match &right.value {
                UntaggedValue::Primitive(Primitive::Int(val)) => {
                    if *val == 0 {
                        Ok(UntaggedValue::Error(ShellError::labeled_error(
                            "Division by zero",
                            "Zero value found",
                            &right.tag.span,
                        )))
                    } else {
                        Ok(compute_series_i64(
                            &lhs,
                            val,
                            <ChunkedArray<Int64Type>>::div,
                            &left.tag.span,
                        ))
                    }
                }
                UntaggedValue::Primitive(Primitive::BigInt(val)) => {
                    if val.eq(&0.into()) {
                        Ok(UntaggedValue::Error(ShellError::labeled_error(
                            "Division by zero",
                            "Zero value found",
                            &right.tag.span,
                        )))
                    } else {
                        Ok(compute_series_i64(
                            &lhs,
                            &val.to_i64()
                                .expect("Internal error: protocol did not use compatible decimal"),
                            <ChunkedArray<Int64Type>>::div,
                            &left.tag.span,
                        ))
                    }
                }
                UntaggedValue::Primitive(Primitive::Decimal(val)) => {
                    if val.eq(&0.into()) {
                        Ok(UntaggedValue::Error(ShellError::labeled_error(
                            "Division by zero",
                            "Zero value found",
                            &right.tag.span,
                        )))
                    } else {
                        Ok(compute_series_decimal(
                            &lhs,
                            val,
                            <ChunkedArray<Float64Type>>::div,
                            &left.tag.span,
                        ))
                    }
                }
                _ => Ok(UntaggedValue::Error(
                    ShellError::labeled_error_with_secondary(
                        "Operation unavailable",
                        "unable to divide this value to the series",
                        &right.tag.span,
                        "Only primary values are allowed",
                        &right.tag.span,
                    ),
                )),
            },
            Operator::Equal => {
                match &right.value {
                    UntaggedValue::Primitive(Primitive::Int(val)) => Ok(compare_series_i64(
                        &lhs,
                        val,
                        ChunkedArray::eq,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compare_series_i64(
                        &lhs,
                        &val.to_i64()
                            .expect("Internal error: protocol did not use compatible decimal"),
                        ChunkedArray::eq,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(
                        compare_series_decimal(&lhs, val, ChunkedArray::eq, &left.tag.span),
                    ),
                    _ => Ok(UntaggedValue::Error(
                        ShellError::labeled_error_with_secondary(
                            "Operation unavailable",
                            "unable to compare this value to the series",
                            &right.tag.span,
                            "Only primary values are allowed",
                            &right.tag.span,
                        ),
                    )),
                }
            }
            Operator::NotEqual => {
                match &right.value {
                    UntaggedValue::Primitive(Primitive::Int(val)) => Ok(compare_series_i64(
                        &lhs,
                        val,
                        ChunkedArray::neq,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compare_series_i64(
                        &lhs,
                        &val.to_i64()
                            .expect("Internal error: protocol did not use compatible decimal"),
                        ChunkedArray::neq,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(
                        compare_series_decimal(&lhs, val, ChunkedArray::neq, &left.tag.span),
                    ),
                    _ => Ok(UntaggedValue::Error(
                        ShellError::labeled_error_with_secondary(
                            "Operation unavailable",
                            "unable to compare this value to the series",
                            &right.tag.span,
                            "Only primary values are allowed",
                            &right.tag.span,
                        ),
                    )),
                }
            }
            Operator::LessThan => {
                match &right.value {
                    UntaggedValue::Primitive(Primitive::Int(val)) => Ok(compare_series_i64(
                        &lhs,
                        val,
                        ChunkedArray::lt,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compare_series_i64(
                        &lhs,
                        &val.to_i64()
                            .expect("Internal error: protocol did not use compatible decimal"),
                        ChunkedArray::lt,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(
                        compare_series_decimal(&lhs, val, ChunkedArray::lt, &left.tag.span),
                    ),
                    _ => Ok(UntaggedValue::Error(
                        ShellError::labeled_error_with_secondary(
                            "Operation unavailable",
                            "unable to compare this value to the series",
                            &right.tag.span,
                            "Only primary values are allowed",
                            &right.tag.span,
                        ),
                    )),
                }
            }
            Operator::LessThanOrEqual => {
                match &right.value {
                    UntaggedValue::Primitive(Primitive::Int(val)) => Ok(compare_series_i64(
                        &lhs,
                        val,
                        ChunkedArray::lt_eq,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compare_series_i64(
                        &lhs,
                        &val.to_i64()
                            .expect("Internal error: protocol did not use compatible decimal"),
                        ChunkedArray::lt_eq,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(
                        compare_series_decimal(&lhs, val, ChunkedArray::lt_eq, &left.tag.span),
                    ),
                    _ => Ok(UntaggedValue::Error(
                        ShellError::labeled_error_with_secondary(
                            "Operation unavailable",
                            "unable to compare this value to the series",
                            &right.tag.span,
                            "Only primary values are allowed",
                            &right.tag.span,
                        ),
                    )),
                }
            }
            Operator::GreaterThan => {
                match &right.value {
                    UntaggedValue::Primitive(Primitive::Int(val)) => Ok(compare_series_i64(
                        &lhs,
                        val,
                        ChunkedArray::gt,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compare_series_i64(
                        &lhs,
                        &val.to_i64()
                            .expect("Internal error: protocol did not use compatible decimal"),
                        ChunkedArray::gt,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(
                        compare_series_decimal(&lhs, val, ChunkedArray::gt, &left.tag.span),
                    ),
                    _ => Ok(UntaggedValue::Error(
                        ShellError::labeled_error_with_secondary(
                            "Operation unavailable",
                            "unable to compare this value to the series",
                            &right.tag.span,
                            "Only primary values are allowed",
                            &right.tag.span,
                        ),
                    )),
                }
            }
            Operator::GreaterThanOrEqual => {
                match &right.value {
                    UntaggedValue::Primitive(Primitive::Int(val)) => Ok(compare_series_i64(
                        &lhs,
                        val,
                        ChunkedArray::gt_eq,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compare_series_i64(
                        &lhs,
                        &val.to_i64()
                            .expect("Internal error: protocol did not use compatible decimal"),
                        ChunkedArray::gt_eq,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(
                        compare_series_decimal(&lhs, val, ChunkedArray::gt_eq, &left.tag.span),
                    ),
                    _ => Ok(UntaggedValue::Error(
                        ShellError::labeled_error_with_secondary(
                            "Operation unavailable",
                            "unable to compare this value to the series",
                            &right.tag.span,
                            "Only primary values are allowed",
                            &right.tag.span,
                        ),
                    )),
                }
            }
            Operator::Contains => match &right.value {
                UntaggedValue::Primitive(Primitive::String(val)) => {
                    Ok(contains_series_pat(&lhs, val, &left.tag.span))
                }
                _ => Ok(UntaggedValue::Error(
                    ShellError::labeled_error_with_secondary(
                        "Operation unavailable",
                        "unable to perform this value to the series",
                        &right.tag.span,
                        "Only primary values are allowed",
                        &right.tag.span,
                    ),
                )),
            },
            _ => Ok(UntaggedValue::Error(ShellError::labeled_error(
                "Incorrect datatype",
                "unable to use this value for this operation",
                &left.tag.span,
            ))),
        }
    } else {
        Err((left.type_name(), right.type_name()))
    }
}

fn compute_series_i64<F>(series: &Series, val: &i64, f: F, span: &Span) -> UntaggedValue
where
    F: Fn(ChunkedArray<Int64Type>, i64) -> ChunkedArray<Int64Type>,
{
    match series.dtype() {
        DataType::UInt32 | DataType::Int32 | DataType::UInt64 => {
            let to_i64 = series.cast_with_dtype(&DataType::Int64);

            match to_i64 {
                Ok(series) => {
                    let casted = series.i64();
                    compute_casted_i64(casted, *val, f, span)
                }
                Err(e) => UntaggedValue::Error(ShellError::labeled_error(
                    "Casting error",
                    e.to_string(),
                    span,
                )),
            }
        }
        DataType::Int64 => {
            let casted = series.i64();
            compute_casted_i64(casted, *val, f, span)
        }
        _ => UntaggedValue::Error(ShellError::labeled_error(
            "Casting error",
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
    span: &Span,
) -> UntaggedValue
where
    F: Fn(ChunkedArray<Int64Type>, i64) -> ChunkedArray<Int64Type>,
{
    match casted {
        Ok(casted) => {
            let res = f(casted.clone(), val);
            let res = res.into_series();
            NuDataFrame::series_to_untagged(res, span)
        }
        Err(e) => UntaggedValue::Error(ShellError::labeled_error(
            "Casting error",
            e.to_string(),
            span,
        )),
    }
}

fn compute_series_decimal<F>(series: &Series, val: &BigDecimal, f: F, span: &Span) -> UntaggedValue
where
    F: Fn(ChunkedArray<Float64Type>, f64) -> ChunkedArray<Float64Type>,
{
    match series.dtype() {
        DataType::Float32 => {
            let to_f64 = series.cast_with_dtype(&DataType::Float64);

            match to_f64 {
                Ok(series) => {
                    let casted = series.f64();
                    compute_casted_f64(
                        casted,
                        val.to_f64()
                            .expect("Internal error: protocol did not use compatible decimal"),
                        f,
                        span,
                    )
                }
                Err(e) => UntaggedValue::Error(ShellError::labeled_error(
                    "Casting error",
                    e.to_string(),
                    span,
                )),
            }
        }
        DataType::Float64 => {
            let casted = series.f64();
            compute_casted_f64(
                casted,
                val.to_f64()
                    .expect("Internal error: protocol did not use compatible decimal"),
                f,
                span,
            )
        }
        _ => UntaggedValue::Error(ShellError::labeled_error(
            "Casting error",
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
    span: &Span,
) -> UntaggedValue
where
    F: Fn(ChunkedArray<Float64Type>, f64) -> ChunkedArray<Float64Type>,
{
    match casted {
        Ok(casted) => {
            let res = f(casted.clone(), val);
            let res = res.into_series();
            NuDataFrame::series_to_untagged(res, span)
        }
        Err(e) => UntaggedValue::Error(ShellError::labeled_error(
            "Casting error",
            e.to_string(),
            span,
        )),
    }
}

fn compare_series_i64<F>(series: &Series, val: &i64, f: F, span: &Span) -> UntaggedValue
where
    F: Fn(&ChunkedArray<Int64Type>, i64) -> ChunkedArray<BooleanType>,
{
    match series.dtype() {
        DataType::UInt32 | DataType::Int32 | DataType::UInt64 => {
            let to_i64 = series.cast_with_dtype(&DataType::Int64);

            match to_i64 {
                Ok(series) => {
                    let casted = series.i64();
                    compare_casted_i64(casted, *val, f, span)
                }
                Err(e) => UntaggedValue::Error(ShellError::labeled_error(
                    "Casting error",
                    e.to_string(),
                    span,
                )),
            }
        }
        DataType::Int64 => {
            let casted = series.i64();
            compare_casted_i64(casted, *val, f, span)
        }
        _ => UntaggedValue::Error(ShellError::labeled_error(
            "Casting error",
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
    span: &Span,
) -> UntaggedValue
where
    F: Fn(&ChunkedArray<Int64Type>, i64) -> ChunkedArray<BooleanType>,
{
    match casted {
        Ok(casted) => {
            let res = f(casted, val);
            let res = res.into_series();
            NuDataFrame::series_to_untagged(res, span)
        }
        Err(e) => UntaggedValue::Error(ShellError::labeled_error(
            "Casting error",
            e.to_string(),
            span,
        )),
    }
}

fn compare_series_decimal<F>(series: &Series, val: &BigDecimal, f: F, span: &Span) -> UntaggedValue
where
    F: Fn(&ChunkedArray<Float64Type>, f64) -> ChunkedArray<BooleanType>,
{
    match series.dtype() {
        DataType::Float32 => {
            let to_f64 = series.cast_with_dtype(&DataType::Float64);

            match to_f64 {
                Ok(series) => {
                    let casted = series.f64();
                    compare_casted_f64(
                        casted,
                        val.to_f64()
                            .expect("Internal error: protocol did not use compatible decimal"),
                        f,
                        span,
                    )
                }
                Err(e) => UntaggedValue::Error(ShellError::labeled_error(
                    "Casting error",
                    e.to_string(),
                    span,
                )),
            }
        }
        DataType::Float64 => {
            let casted = series.f64();
            compare_casted_f64(
                casted,
                val.to_f64()
                    .expect("Internal error: protocol did not use compatible decimal"),
                f,
                span,
            )
        }
        _ => UntaggedValue::Error(ShellError::labeled_error(
            "Casting error",
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
    span: &Span,
) -> UntaggedValue
where
    F: Fn(&ChunkedArray<Float64Type>, f64) -> ChunkedArray<BooleanType>,
{
    match casted {
        Ok(casted) => {
            let res = f(casted, val);
            let res = res.into_series();
            NuDataFrame::series_to_untagged(res, span)
        }
        Err(e) => UntaggedValue::Error(ShellError::labeled_error(
            "Casting error",
            e.to_string(),
            span,
        )),
    }
}

fn contains_series_pat(series: &Series, pat: &str, span: &Span) -> UntaggedValue {
    let casted = series.utf8();
    match casted {
        Ok(casted) => {
            let res = casted.contains(pat);

            match res {
                Ok(res) => {
                    let res = res.into_series();
                    NuDataFrame::series_to_untagged(res, span)
                }
                Err(e) => UntaggedValue::Error(ShellError::labeled_error(
                    "Search error",
                    e.to_string(),
                    span,
                )),
            }
        }
        Err(e) => UntaggedValue::Error(ShellError::labeled_error(
            "Casting error",
            e.to_string(),
            span,
        )),
    }
}
