use bigdecimal::BigDecimal;
use nu_errors::ShellError;
use nu_protocol::hir::Operator;
use nu_protocol::{
    dataframe::{NuSeries, PolarsData},
    Primitive, ShellTypeName, UntaggedValue, Value,
};
use nu_source::Span;
use num_traits::ToPrimitive;

use num_bigint::BigInt;
use polars::prelude::{
    BooleanType, ChunkCompare, ChunkedArray, DataType, Float64Type, Int64Type, IntoSeries,
    NumOpsDispatchChecked, Series,
};
use std::ops::{Add, BitAnd, BitOr, Div, Mul, Sub};

pub fn compute_between_series(
    operator: Operator,
    left: &Value,
    right: &Value,
) -> Result<UntaggedValue, (&'static str, &'static str)> {
    if let (
        UntaggedValue::DataFrame(PolarsData::Series(lhs)),
        UntaggedValue::DataFrame(PolarsData::Series(rhs)),
    ) = (&left.value, &right.value)
    {
        if lhs.as_ref().dtype() != rhs.as_ref().dtype() {
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

        if lhs.as_ref().len() != rhs.as_ref().len() {
            return Ok(UntaggedValue::Error(ShellError::labeled_error(
                "Different length",
                "this column length does not match the right hand column length",
                &left.tag.span,
            )));
        }

        match operator {
            Operator::Plus => {
                let mut res = lhs.as_ref() + rhs.as_ref();
                let name = format!("sum_{}_{}", lhs.as_ref().name(), rhs.as_ref().name());
                res.rename(name.as_ref());
                Ok(NuSeries::series_to_untagged(res))
            }
            Operator::Minus => {
                let mut res = lhs.as_ref() - rhs.as_ref();
                let name = format!("sub_{}_{}", lhs.as_ref().name(), rhs.as_ref().name());
                res.rename(name.as_ref());
                Ok(NuSeries::series_to_untagged(res))
            }
            Operator::Multiply => {
                let mut res = lhs.as_ref() * rhs.as_ref();
                let name = format!("mul_{}_{}", lhs.as_ref().name(), rhs.as_ref().name());
                res.rename(name.as_ref());
                Ok(NuSeries::series_to_untagged(res))
            }
            Operator::Divide => {
                let res = lhs.as_ref().checked_div(rhs.as_ref());
                match res {
                    Ok(mut res) => {
                        let name = format!("div_{}_{}", lhs.as_ref().name(), rhs.as_ref().name());
                        res.rename(name.as_ref());
                        Ok(NuSeries::series_to_untagged(res))
                    }
                    Err(e) => Ok(UntaggedValue::Error(ShellError::labeled_error(
                        "Division error",
                        format!("{}", e),
                        &left.tag.span,
                    ))),
                }
            }
            Operator::Equal => {
                let mut res = Series::eq(lhs.as_ref(), rhs.as_ref()).into_series();
                let name = format!("eq_{}_{}", lhs.as_ref().name(), rhs.as_ref().name());
                res.rename(name.as_ref());
                Ok(NuSeries::series_to_untagged(res))
            }
            Operator::NotEqual => {
                let mut res = Series::neq(lhs.as_ref(), rhs.as_ref()).into_series();
                let name = format!("neq_{}_{}", lhs.as_ref().name(), rhs.as_ref().name());
                res.rename(name.as_ref());
                Ok(NuSeries::series_to_untagged(res))
            }
            Operator::LessThan => {
                let mut res = Series::lt(lhs.as_ref(), rhs.as_ref()).into_series();
                let name = format!("lt_{}_{}", lhs.as_ref().name(), rhs.as_ref().name());
                res.rename(name.as_ref());
                Ok(NuSeries::series_to_untagged(res))
            }
            Operator::LessThanOrEqual => {
                let mut res = Series::lt_eq(lhs.as_ref(), rhs.as_ref()).into_series();
                let name = format!("lte_{}_{}", lhs.as_ref().name(), rhs.as_ref().name());
                res.rename(name.as_ref());
                Ok(NuSeries::series_to_untagged(res))
            }
            Operator::GreaterThan => {
                let mut res = Series::gt(lhs.as_ref(), rhs.as_ref()).into_series();
                let name = format!("gt_{}_{}", lhs.as_ref().name(), rhs.as_ref().name());
                res.rename(name.as_ref());
                Ok(NuSeries::series_to_untagged(res))
            }
            Operator::GreaterThanOrEqual => {
                let mut res = Series::gt_eq(lhs.as_ref(), rhs.as_ref()).into_series();
                let name = format!("gte_{}_{}", lhs.as_ref().name(), rhs.as_ref().name());
                res.rename(name.as_ref());
                Ok(NuSeries::series_to_untagged(res))
            }
            Operator::And => match lhs.as_ref().dtype() {
                DataType::Boolean => {
                    let lhs_cast = lhs.as_ref().bool();
                    let rhs_cast = rhs.as_ref().bool();

                    match (lhs_cast, rhs_cast) {
                        (Ok(l), Ok(r)) => {
                            let mut res = l.bitand(r).into_series();
                            let name =
                                format!("and_{}_{}", lhs.as_ref().name(), rhs.as_ref().name());
                            res.rename(name.as_ref());
                            Ok(NuSeries::series_to_untagged(res))
                        }
                        _ => Ok(UntaggedValue::Error(
                            ShellError::labeled_error_with_secondary(
                                "Casting error",
                                "unable to cast to boolean",
                                &left.tag.span,
                                "unable to cast to boolean",
                                &right.tag.span,
                            ),
                        )),
                    }
                }
                _ => Ok(UntaggedValue::Error(ShellError::labeled_error(
                    "Incorrect datatype",
                    "And operation can only be done with boolean values",
                    &left.tag.span,
                ))),
            },
            Operator::Or => match lhs.as_ref().dtype() {
                DataType::Boolean => {
                    let lhs_cast = lhs.as_ref().bool();
                    let rhs_cast = rhs.as_ref().bool();

                    match (lhs_cast, rhs_cast) {
                        (Ok(l), Ok(r)) => {
                            let mut res = l.bitor(r).into_series();
                            let name =
                                format!("or_{}_{}", lhs.as_ref().name(), rhs.as_ref().name());
                            res.rename(name.as_ref());
                            Ok(NuSeries::series_to_untagged(res))
                        }
                        _ => Ok(UntaggedValue::Error(
                            ShellError::labeled_error_with_secondary(
                                "Casting error",
                                "unable to cast to boolean",
                                &left.tag.span,
                                "unable to cast to boolean",
                                &right.tag.span,
                            ),
                        )),
                    }
                }
                _ => Ok(UntaggedValue::Error(ShellError::labeled_error(
                    "Incorrect datatype",
                    "And operation can only be done with boolean values",
                    &left.tag.span,
                ))),
            },
            _ => Ok(UntaggedValue::Error(ShellError::labeled_error(
                "Incorrect datatype",
                "unable to use this datatype for this operation",
                &left.tag.span,
            ))),
        }
    } else {
        Err((left.type_name(), right.type_name()))
    }
}

pub fn compute_series_single_value(
    operator: Operator,
    left: &Value,
    right: &Value,
) -> Result<UntaggedValue, (&'static str, &'static str)> {
    if let (UntaggedValue::DataFrame(PolarsData::Series(lhs)), UntaggedValue::Primitive(_)) =
        (&left.value, &right.value)
    {
        match operator {
            Operator::Plus => match &right.value {
                UntaggedValue::Primitive(Primitive::Int(val)) => Ok(compute_series_i64(
                    lhs.as_ref(),
                    val,
                    <&ChunkedArray<Int64Type>>::add,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compute_series_bigint(
                    lhs.as_ref(),
                    val,
                    <&ChunkedArray<Int64Type>>::add,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(compute_series_decimal(
                    lhs.as_ref(),
                    val,
                    <&ChunkedArray<Float64Type>>::add,
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
                    lhs.as_ref(),
                    val,
                    <&ChunkedArray<Int64Type>>::sub,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compute_series_bigint(
                    lhs.as_ref(),
                    val,
                    <&ChunkedArray<Int64Type>>::sub,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(compute_series_decimal(
                    lhs.as_ref(),
                    val,
                    <&ChunkedArray<Float64Type>>::sub,
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
                    lhs.as_ref(),
                    val,
                    <&ChunkedArray<Int64Type>>::mul,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compute_series_bigint(
                    lhs.as_ref(),
                    val,
                    <&ChunkedArray<Int64Type>>::mul,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(compute_series_decimal(
                    lhs.as_ref(),
                    val,
                    <&ChunkedArray<Float64Type>>::mul,
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
                            lhs.as_ref(),
                            val,
                            <&ChunkedArray<Int64Type>>::div,
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
                        Ok(compute_series_bigint(
                            lhs.as_ref(),
                            val,
                            <&ChunkedArray<Int64Type>>::div,
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
                            lhs.as_ref(),
                            val,
                            <&ChunkedArray<Float64Type>>::div,
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
                        lhs.as_ref(),
                        val,
                        ChunkedArray::eq,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compare_series_bigint(
                        lhs.as_ref(),
                        val,
                        ChunkedArray::eq,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(
                        compare_series_decimal(lhs.as_ref(), val, ChunkedArray::eq, &left.tag.span),
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
            Operator::NotEqual => match &right.value {
                UntaggedValue::Primitive(Primitive::Int(val)) => Ok(compare_series_i64(
                    lhs.as_ref(),
                    val,
                    ChunkedArray::neq,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compare_series_bigint(
                    lhs.as_ref(),
                    val,
                    ChunkedArray::neq,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(compare_series_decimal(
                    lhs.as_ref(),
                    val,
                    ChunkedArray::neq,
                    &left.tag.span,
                )),
                _ => Ok(UntaggedValue::Error(
                    ShellError::labeled_error_with_secondary(
                        "Operation unavailable",
                        "unable to compare this value to the series",
                        &right.tag.span,
                        "Only primary values are allowed",
                        &right.tag.span,
                    ),
                )),
            },
            Operator::LessThan => {
                match &right.value {
                    UntaggedValue::Primitive(Primitive::Int(val)) => Ok(compare_series_i64(
                        lhs.as_ref(),
                        val,
                        ChunkedArray::lt,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compare_series_bigint(
                        lhs.as_ref(),
                        val,
                        ChunkedArray::lt,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(
                        compare_series_decimal(lhs.as_ref(), val, ChunkedArray::lt, &left.tag.span),
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
            Operator::LessThanOrEqual => match &right.value {
                UntaggedValue::Primitive(Primitive::Int(val)) => Ok(compare_series_i64(
                    lhs.as_ref(),
                    val,
                    ChunkedArray::lt_eq,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compare_series_bigint(
                    lhs.as_ref(),
                    val,
                    ChunkedArray::lt_eq,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(compare_series_decimal(
                    lhs.as_ref(),
                    val,
                    ChunkedArray::lt_eq,
                    &left.tag.span,
                )),
                _ => Ok(UntaggedValue::Error(
                    ShellError::labeled_error_with_secondary(
                        "Operation unavailable",
                        "unable to compare this value to the series",
                        &right.tag.span,
                        "Only primary values are allowed",
                        &right.tag.span,
                    ),
                )),
            },
            Operator::GreaterThan => {
                match &right.value {
                    UntaggedValue::Primitive(Primitive::Int(val)) => Ok(compare_series_i64(
                        lhs.as_ref(),
                        val,
                        ChunkedArray::gt,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compare_series_bigint(
                        lhs.as_ref(),
                        val,
                        ChunkedArray::gt,
                        &left.tag.span,
                    )),
                    UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(
                        compare_series_decimal(lhs.as_ref(), val, ChunkedArray::gt, &left.tag.span),
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
            Operator::GreaterThanOrEqual => match &right.value {
                UntaggedValue::Primitive(Primitive::Int(val)) => Ok(compare_series_i64(
                    lhs.as_ref(),
                    val,
                    ChunkedArray::gt_eq,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::BigInt(val)) => Ok(compare_series_bigint(
                    lhs.as_ref(),
                    val,
                    ChunkedArray::gt_eq,
                    &left.tag.span,
                )),
                UntaggedValue::Primitive(Primitive::Decimal(val)) => Ok(compare_series_decimal(
                    lhs.as_ref(),
                    val,
                    ChunkedArray::gt_eq,
                    &left.tag.span,
                )),
                _ => Ok(UntaggedValue::Error(
                    ShellError::labeled_error_with_secondary(
                        "Operation unavailable",
                        "unable to compare this value to the series",
                        &right.tag.span,
                        "Only primary values are allowed",
                        &right.tag.span,
                    ),
                )),
            },
            Operator::Contains => match &right.value {
                UntaggedValue::Primitive(Primitive::String(val)) => {
                    Ok(contains_series_pat(lhs.as_ref(), val, &left.tag.span))
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

fn compute_series_i64<'r, F>(series: &'r Series, val: &i64, f: F, span: &Span) -> UntaggedValue
where
    F: Fn(&'r ChunkedArray<Int64Type>, i64) -> ChunkedArray<Int64Type>,
{
    let casted = series.i64();
    match casted {
        Ok(casted) => {
            let res = f(casted, *val);
            let res = res.into_series();
            NuSeries::series_to_untagged(res)
        }
        Err(e) => UntaggedValue::Error(ShellError::labeled_error(
            "Casting error",
            format!("{}", e),
            span,
        )),
    }
}

fn compute_series_bigint<'r, F>(
    series: &'r Series,
    val: &BigInt,
    f: F,
    span: &Span,
) -> UntaggedValue
where
    F: Fn(&'r ChunkedArray<Int64Type>, i64) -> ChunkedArray<Int64Type>,
{
    let casted = series.i64();
    match casted {
        Ok(casted) => {
            let res = f(
                casted,
                val.to_i64()
                    .expect("Internal error: protocol did not use compatible decimal"),
            );
            let res = res.into_series();
            NuSeries::series_to_untagged(res)
        }
        Err(e) => UntaggedValue::Error(ShellError::labeled_error(
            "Casting error",
            format!("{}", e),
            span,
        )),
    }
}

fn compute_series_decimal<'r, F>(
    series: &'r Series,
    val: &BigDecimal,
    f: F,
    span: &Span,
) -> UntaggedValue
where
    F: Fn(&'r ChunkedArray<Float64Type>, f64) -> ChunkedArray<Float64Type>,
{
    let casted = series.f64();
    match casted {
        Ok(casted) => {
            let res = f(
                casted,
                val.to_f64()
                    .expect("Internal error: protocol did not use compatible decimal"),
            );
            let res = res.into_series();
            NuSeries::series_to_untagged(res)
        }
        Err(e) => UntaggedValue::Error(ShellError::labeled_error(
            "Casting error",
            format!("{}", e),
            span,
        )),
    }
}

fn compare_series_i64<'r, F>(series: &'r Series, val: &i64, f: F, span: &Span) -> UntaggedValue
where
    F: Fn(&'r ChunkedArray<Int64Type>, i64) -> ChunkedArray<BooleanType>,
{
    let casted = series.i64();
    match casted {
        Ok(casted) => {
            let res = f(casted, *val);
            let res = res.into_series();
            NuSeries::series_to_untagged(res)
        }
        Err(e) => UntaggedValue::Error(ShellError::labeled_error(
            "Casting error",
            format!("{}", e),
            span,
        )),
    }
}

fn compare_series_bigint<'r, F>(
    series: &'r Series,
    val: &BigInt,
    f: F,
    span: &Span,
) -> UntaggedValue
where
    F: Fn(&'r ChunkedArray<Int64Type>, i64) -> ChunkedArray<BooleanType>,
{
    let casted = series.i64();
    match casted {
        Ok(casted) => {
            let res = f(
                casted,
                val.to_i64()
                    .expect("Internal error: protocol did not use compatible decimal"),
            );
            let res = res.into_series();
            NuSeries::series_to_untagged(res)
        }
        Err(e) => UntaggedValue::Error(ShellError::labeled_error(
            "Casting error",
            format!("{}", e),
            span,
        )),
    }
}

fn compare_series_decimal<'r, F>(
    series: &'r Series,
    val: &BigDecimal,
    f: F,
    span: &Span,
) -> UntaggedValue
where
    F: Fn(&'r ChunkedArray<Float64Type>, i64) -> ChunkedArray<BooleanType>,
{
    let casted = series.f64();
    match casted {
        Ok(casted) => {
            let res = f(
                casted,
                val.to_i64()
                    .expect("Internal error: protocol did not use compatible decimal"),
            );
            let res = res.into_series();
            NuSeries::series_to_untagged(res)
        }
        Err(e) => UntaggedValue::Error(ShellError::labeled_error(
            "Casting error",
            format!("{}", e),
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
                    NuSeries::series_to_untagged(res)
                }
                Err(e) => UntaggedValue::Error(ShellError::labeled_error(
                    "Search error",
                    format!("{}", e),
                    span,
                )),
            }
        }
        Err(e) => UntaggedValue::Error(ShellError::labeled_error(
            "Casting error",
            format!("{}", e),
            span,
        )),
    }
}
