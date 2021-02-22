use crate::base::coerce_compare;
use crate::base::shape::{Column, InlineShape};
use crate::primitive::style_primitive;
use chrono::{DateTime, NaiveDate, Utc};
use nu_errors::ShellError;
use nu_protocol::hir::Operator;
use nu_protocol::ShellTypeName;
use nu_protocol::{Primitive, Type, UntaggedValue};
use nu_source::{DebugDocBuilder, PrettyDebug, Span, Tagged};
use nu_table::TextStyle;
use num_traits::{ToPrimitive, Zero};
use std::collections::HashMap;

pub struct Date;

impl Date {
    pub fn from_regular_str(s: Tagged<&str>) -> Result<UntaggedValue, ShellError> {
        let date = DateTime::parse_from_rfc3339(s.item).map_err(|err| {
            ShellError::labeled_error(
                &format!("Date parse error: {}", err),
                "original value",
                s.tag,
            )
        })?;

        let date = date.with_timezone(&chrono::offset::Utc);

        Ok(UntaggedValue::Primitive(Primitive::Date(date.into())))
    }

    pub fn naive_from_str(s: Tagged<&str>) -> Result<UntaggedValue, ShellError> {
        let date = NaiveDate::parse_from_str(s.item, "%Y-%m-%d").map_err(|reason| {
            ShellError::labeled_error(
                &format!("Date parse error: {}", reason),
                "original value",
                s.tag,
            )
        })?;

        Ok(UntaggedValue::Primitive(Primitive::Date(
            DateTime::<Utc>::from_utc(date.and_hms(12, 34, 56), Utc).into(),
        )))
    }
}

pub fn date_from_str(s: Tagged<&str>) -> Result<UntaggedValue, ShellError> {
    Date::from_regular_str(s)
}

pub fn date_naive_from_str(s: Tagged<&str>) -> Result<UntaggedValue, ShellError> {
    Date::naive_from_str(s)
}

pub fn merge_values(
    left: &UntaggedValue,
    right: &UntaggedValue,
) -> Result<UntaggedValue, (&'static str, &'static str)> {
    match (left, right) {
        (UntaggedValue::Row(columns), UntaggedValue::Row(columns_b)) => {
            Ok(UntaggedValue::Row(columns.merge_from(columns_b)))
        }
        (left, right) => Err((left.type_name(), right.type_name())),
    }
}

fn zero_division_error() -> UntaggedValue {
    UntaggedValue::Error(ShellError::untagged_runtime_error("division by zero"))
}

pub fn unsafe_compute_values(
    operator: Operator,
    left: &UntaggedValue,
    right: &UntaggedValue,
) -> Result<UntaggedValue, (&'static str, &'static str)> {
    let computed = compute_values(operator, left, right);

    if computed.is_ok() {
        return computed;
    }

    match (left, right) {
        (UntaggedValue::Primitive(lhs), UntaggedValue::Primitive(rhs)) => match (lhs, rhs) {
            (Primitive::Filesize(x), Primitive::Int(y)) => match operator {
                Operator::Multiply => Ok(UntaggedValue::Primitive(Primitive::Filesize(x * y))),
                Operator::Divide => Ok(UntaggedValue::Primitive(Primitive::Filesize(x / y))),
                _ => Err((left.type_name(), right.type_name())),
            },
            (Primitive::Int(x), Primitive::Filesize(y)) => match operator {
                Operator::Multiply => Ok(UntaggedValue::Primitive(Primitive::Filesize(x * y))),
                _ => Err((left.type_name(), right.type_name())),
            },
            _ => Err((left.type_name(), right.type_name())),
        },
        _ => Err((left.type_name(), right.type_name())),
    }
}

pub fn compute_values(
    operator: Operator,
    left: &UntaggedValue,
    right: &UntaggedValue,
) -> Result<UntaggedValue, (&'static str, &'static str)> {
    match (left, right) {
        (UntaggedValue::Primitive(lhs), UntaggedValue::Primitive(rhs)) => match (lhs, rhs) {
            (Primitive::Filesize(x), Primitive::Filesize(y)) => {
                let result = match operator {
                    Operator::Plus => Ok(x + y),
                    Operator::Minus => Ok(x - y),
                    _ => Err((left.type_name(), right.type_name())),
                }?;
                Ok(UntaggedValue::Primitive(Primitive::Filesize(result)))
            }
            (Primitive::Filesize(x), Primitive::Int(y)) => match operator {
                Operator::Multiply => Ok(UntaggedValue::Primitive(Primitive::Filesize(x * y))),
                Operator::Divide => Ok(UntaggedValue::Primitive(Primitive::Filesize(x / y))),
                _ => Err((left.type_name(), right.type_name())),
            },
            (Primitive::Int(x), Primitive::Filesize(y)) => match operator {
                Operator::Multiply => Ok(UntaggedValue::Primitive(Primitive::Filesize(x * y))),
                _ => Err((left.type_name(), right.type_name())),
            },
            (Primitive::Int(x), Primitive::Int(y)) => match operator {
                Operator::Plus => Ok(UntaggedValue::Primitive(Primitive::Int(x + y))),
                Operator::Minus => Ok(UntaggedValue::Primitive(Primitive::Int(x - y))),
                Operator::Multiply => Ok(UntaggedValue::Primitive(Primitive::Int(x * y))),
                Operator::Divide => {
                    if y.is_zero() {
                        Ok(zero_division_error())
                    } else if x - (y * (x / y)) == num_bigint::BigInt::from(0) {
                        Ok(UntaggedValue::Primitive(Primitive::Int(x / y)))
                    } else {
                        Ok(UntaggedValue::Primitive(Primitive::Decimal(
                            bigdecimal::BigDecimal::from(x.clone())
                                / bigdecimal::BigDecimal::from(y.clone()),
                        )))
                    }
                }
                Operator::Modulo => {
                    if y.is_zero() {
                        Ok(zero_division_error())
                    } else {
                        Ok(UntaggedValue::Primitive(Primitive::Int(x % y)))
                    }
                }
                Operator::Pow => {
                    let prim_u32 = ToPrimitive::to_u32(y);
                    match prim_u32 {
                        Some(num) => Ok(UntaggedValue::Primitive(Primitive::Int(x.pow(num)))),
                        _ => Err((left.type_name(), right.type_name())),
                    }
                }
                _ => Err((left.type_name(), right.type_name())),
            },
            (Primitive::Decimal(x), Primitive::Int(y)) => {
                let result = match operator {
                    Operator::Plus => Ok(x + bigdecimal::BigDecimal::from(y.clone())),
                    Operator::Minus => Ok(x - bigdecimal::BigDecimal::from(y.clone())),
                    Operator::Multiply => Ok(x * bigdecimal::BigDecimal::from(y.clone())),
                    Operator::Divide => {
                        if y.is_zero() {
                            return Ok(zero_division_error());
                        }
                        Ok(x / bigdecimal::BigDecimal::from(y.clone()))
                    }
                    Operator::Modulo => {
                        if y.is_zero() {
                            return Ok(zero_division_error());
                        }
                        Ok(x % bigdecimal::BigDecimal::from(y.clone()))
                    }
                    // leaving this here for the hope that bigdecimal will one day support pow/powf/fpow
                    // Operator::Pow => {
                    //     let xp = bigdecimal::ToPrimitive::to_f64(x).unwrap_or(0.0);
                    //     let yp = bigdecimal::ToPrimitive::to_f64(y).unwrap_or(0.0);
                    //     let pow = bigdecimal::FromPrimitive::from_f64(xp.powf(yp));
                    //     match pow {
                    //         Some(p) => Ok(p),
                    //         None => Err((left.type_name(), right.type_name())),
                    //     }
                    // }
                    _ => Err((left.type_name(), right.type_name())),
                }?;
                Ok(UntaggedValue::Primitive(Primitive::Decimal(result)))
            }
            (Primitive::Int(x), Primitive::Decimal(y)) => {
                let result = match operator {
                    Operator::Plus => Ok(bigdecimal::BigDecimal::from(x.clone()) + y),
                    Operator::Minus => Ok(bigdecimal::BigDecimal::from(x.clone()) - y),
                    Operator::Multiply => Ok(bigdecimal::BigDecimal::from(x.clone()) * y),
                    Operator::Divide => {
                        if y.is_zero() {
                            return Ok(zero_division_error());
                        }
                        Ok(bigdecimal::BigDecimal::from(x.clone()) / y)
                    }
                    Operator::Modulo => {
                        if y.is_zero() {
                            return Ok(zero_division_error());
                        }
                        Ok(bigdecimal::BigDecimal::from(x.clone()) % y)
                    }
                    // big decimal doesn't support pow yet
                    // Operator::Pow => {
                    //     let yp = bigdecimal::ToPrimitive::to_u32(y).unwrap_or(0);
                    //     Ok(bigdecimal::BigDecimal::from(x.pow(yp)))
                    // }
                    _ => Err((left.type_name(), right.type_name())),
                }?;
                Ok(UntaggedValue::Primitive(Primitive::Decimal(result)))
            }
            (Primitive::Decimal(x), Primitive::Decimal(y)) => {
                let result = match operator {
                    Operator::Plus => Ok(x + y),
                    Operator::Minus => Ok(x - y),
                    Operator::Multiply => Ok(x * y),
                    Operator::Divide => {
                        if y.is_zero() {
                            return Ok(zero_division_error());
                        }
                        Ok(x / y)
                    }
                    Operator::Modulo => {
                        if y.is_zero() {
                            return Ok(zero_division_error());
                        }
                        Ok(x % y)
                    }
                    // big decimal doesn't support pow yet
                    // Operator::Pow => {
                    //     let xp = bigdecimal::ToPrimitive::to_f64(x).unwrap_or(0.0);
                    //     let yp = bigdecimal::ToPrimitive::to_f64(y).unwrap_or(0.0);
                    //     let pow = bigdecimal::FromPrimitive::from_f64(xp.powf(yp));
                    //     match pow {
                    //         Some(p) => Ok(p),
                    //         None => Err((left.type_name(), right.type_name())),
                    //     }
                    // }
                    _ => Err((left.type_name(), right.type_name())),
                }?;
                Ok(UntaggedValue::Primitive(Primitive::Decimal(result)))
            }
            (Primitive::Date(x), Primitive::Date(y)) => match operator {
                Operator::Minus => Ok(UntaggedValue::Primitive(Primitive::from(
                    x.signed_duration_since(*y),
                ))),
                _ => Err((left.type_name(), right.type_name())),
            },
            (Primitive::Date(x), Primitive::Duration(_)) => {
                let result = match operator {
                    Operator::Plus => {
                        // FIXME: Not sure if I could do something better with the Span.
                        match Primitive::into_chrono_duration(rhs.clone(), Span::unknown()) {
                            Ok(y) => match x.checked_add_signed(y) {
                                Some(value) => Ok(value),
                                None => Err(("Date", "Duration and date addition overflow")),
                            },
                            Err(_) => Err(("Date", "Duration overflow")),
                        }
                    }
                    _ => Err((left.type_name(), right.type_name())),
                }?;
                Ok(UntaggedValue::Primitive(Primitive::Date(result)))
            }
            (Primitive::Duration(x), Primitive::Duration(y)) => {
                let result = match operator {
                    Operator::Plus => Ok(x + y),
                    Operator::Minus => Ok(x - y),
                    _ => Err((left.type_name(), right.type_name())),
                }?;

                Ok(UntaggedValue::Primitive(Primitive::Duration(result)))
            }
            (Primitive::Int(x), Primitive::Duration(y)) => {
                let result = match operator {
                    Operator::Plus => Ok(x + y),
                    Operator::Minus => Ok(x - y),
                    _ => Err((left.type_name(), right.type_name())),
                }?;

                Ok(UntaggedValue::Primitive(Primitive::Duration(result)))
            }
            (Primitive::Duration(x), Primitive::Decimal(y)) => {
                let result = match operator {
                    Operator::Divide => {
                        if y.is_zero() {
                            return Ok(zero_division_error());
                        }
                        let y = y.as_bigint_and_exponent();
                        Ok(x / y.0)
                    }
                    _ => Err((left.type_name(), right.type_name())),
                }?;

                Ok(UntaggedValue::Primitive(Primitive::Duration(result)))
            }
            _ => Err((left.type_name(), right.type_name())),
        },
        _ => Err((left.type_name(), right.type_name())),
    }
}

/// If left is {{ Operator }} right
pub fn compare_values(
    operator: Operator,
    left: &UntaggedValue,
    right: &UntaggedValue,
) -> Result<bool, (&'static str, &'static str)> {
    let coerced = coerce_compare(left, right)?;
    let ordering = coerced.compare();

    use std::cmp::Ordering;

    let result = matches!(
        (operator, ordering),
        (Operator::Equal, Ordering::Equal)
            | (Operator::GreaterThan, Ordering::Greater)
            | (Operator::GreaterThanOrEqual, Ordering::Greater)
            | (Operator::GreaterThanOrEqual, Ordering::Equal)
            | (Operator::LessThan, Ordering::Less)
            | (Operator::LessThanOrEqual, Ordering::Less)
            | (Operator::LessThanOrEqual, Ordering::Equal)
            | (Operator::NotEqual, Ordering::Greater)
            | (Operator::NotEqual, Ordering::Less)
    );

    Ok(result)
}

pub fn format_type<'a>(value: impl Into<&'a UntaggedValue>, width: usize) -> String {
    Type::from_value(value.into()).colored_string(width)
}

pub fn format_leaf<'a>(value: impl Into<&'a UntaggedValue>) -> DebugDocBuilder {
    InlineShape::from_value(value.into()).format().pretty()
}

pub fn style_leaf<'a>(
    value: impl Into<&'a UntaggedValue>,
    color_hash_map: &HashMap<String, nu_ansi_term::Style>,
) -> TextStyle {
    match value.into() {
        UntaggedValue::Primitive(p) => {
            // This is just to return the name of the type so that style_primitive
            // can work on a string versus a type like String("some_text")
            let str: &str = &p.to_string();
            let str_len = str.len();
            let paren_index = str.find('(').unwrap_or(str_len - 1);
            let prim_type = str[0..paren_index].to_string();
            style_primitive(&prim_type, &color_hash_map)
        }
        _ => TextStyle::basic_left(),
    }
}

pub fn format_for_column<'a>(
    value: impl Into<&'a UntaggedValue>,
    column: impl Into<Column>,
) -> DebugDocBuilder {
    InlineShape::from_value(value.into())
        .format_for_column(column)
        .pretty()
}

#[cfg(test)]
mod tests {
    use super::merge_values;
    use super::Date as d;
    use super::UntaggedValue as v;
    use nu_source::TaggedItem;

    use indexmap::indexmap;

    #[test]
    fn merges_tables() {
        let (author_1_date, author_2_date) = (
            "2020-04-29".to_string().tagged_unknown(),
            "2019-10-10".to_string().tagged_unknown(),
        );

        let table_author_row = v::row(indexmap! {
            "name".into() => v::string("Andrés").into_untagged_value(),
            "country".into() => v::string("EC").into_untagged_value(),
            "date".into() => d::naive_from_str(author_1_date.borrow_tagged()).unwrap().into_untagged_value()
        });

        let other_table_author_row = v::row(indexmap! {
            "name".into() => v::string("YK").into_untagged_value(),
            "country".into() => v::string("US").into_untagged_value(),
            "date".into() => d::naive_from_str(author_2_date.borrow_tagged()).unwrap().into_untagged_value()
        });

        assert_eq!(
            other_table_author_row,
            merge_values(&table_author_row, &other_table_author_row).unwrap()
        );
    }
}
