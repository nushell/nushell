use crate::data::base::coerce_compare;
use crate::data::base::shape::{Column, InlineShape};
use crate::data::primitive::style_primitive;
use chrono::DateTime;
use nu_errors::ShellError;
use nu_protocol::{hir::Operator, Primitive, ShellTypeName, Type, UntaggedValue, Value};
use nu_source::{DebugDocBuilder, PrettyDebug, Tagged};
use prettytable::color;

pub fn date_from_str(s: Tagged<&str>) -> Result<UntaggedValue, ShellError> {
    let date = DateTime::parse_from_rfc3339(s.item).map_err(|err| {
        ShellError::labeled_error(
            &format!("Date parse error: {}", err),
            "original value",
            s.tag,
        )
    })?;

    let date = date.with_timezone(&chrono::offset::Utc);

    Ok(UntaggedValue::Primitive(Primitive::Date(date)))
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

pub fn compute_values(
    operator: Operator,
    left: &UntaggedValue,
    right: &UntaggedValue,
) -> Result<UntaggedValue, (&'static str, &'static str)> {
    match (left, right) {
        (UntaggedValue::Primitive(lhs), UntaggedValue::Primitive(rhs)) => match (lhs, rhs) {
            (Primitive::Bytes(x), Primitive::Bytes(y)) => {
                let result = match operator {
                    Operator::Plus => Ok(x + y),
                    Operator::Minus => Ok(x - y),
                    _ => Err((left.type_name(), right.type_name())),
                }?;
                Ok(UntaggedValue::Primitive(Primitive::Bytes(result)))
            }
            (Primitive::Int(x), Primitive::Int(y)) => match operator {
                Operator::Plus => Ok(UntaggedValue::Primitive(Primitive::Int(x + y))),
                Operator::Minus => Ok(UntaggedValue::Primitive(Primitive::Int(x - y))),
                Operator::Multiply => Ok(UntaggedValue::Primitive(Primitive::Int(x * y))),
                Operator::Divide => {
                    if x - (y * (x / y)) == num_bigint::BigInt::from(0) {
                        Ok(UntaggedValue::Primitive(Primitive::Int(x / y)))
                    } else {
                        Ok(UntaggedValue::Primitive(Primitive::Decimal(
                            bigdecimal::BigDecimal::from(x.clone())
                                / bigdecimal::BigDecimal::from(y.clone()),
                        )))
                    }
                }
                _ => Err((left.type_name(), right.type_name())),
            },
            (Primitive::Decimal(x), Primitive::Int(y)) => {
                let result = match operator {
                    Operator::Plus => Ok(x + bigdecimal::BigDecimal::from(y.clone())),
                    Operator::Minus => Ok(x - bigdecimal::BigDecimal::from(y.clone())),
                    Operator::Multiply => Ok(x * bigdecimal::BigDecimal::from(y.clone())),
                    Operator::Divide => Ok(x / bigdecimal::BigDecimal::from(y.clone())),
                    _ => Err((left.type_name(), right.type_name())),
                }?;
                Ok(UntaggedValue::Primitive(Primitive::Decimal(result)))
            }
            (Primitive::Int(x), Primitive::Decimal(y)) => {
                let result = match operator {
                    Operator::Plus => Ok(bigdecimal::BigDecimal::from(x.clone()) + y),
                    Operator::Minus => Ok(bigdecimal::BigDecimal::from(x.clone()) - y),
                    Operator::Multiply => Ok(bigdecimal::BigDecimal::from(x.clone()) * y),
                    Operator::Divide => Ok(bigdecimal::BigDecimal::from(x.clone()) / y),
                    _ => Err((left.type_name(), right.type_name())),
                }?;
                Ok(UntaggedValue::Primitive(Primitive::Decimal(result)))
            }
            (Primitive::Decimal(x), Primitive::Decimal(y)) => {
                let result = match operator {
                    Operator::Plus => Ok(x + y),
                    Operator::Minus => Ok(x - y),
                    Operator::Multiply => Ok(x * y),
                    Operator::Divide => Ok(x / y),
                    _ => Err((left.type_name(), right.type_name())),
                }?;
                Ok(UntaggedValue::Primitive(Primitive::Decimal(result)))
            }
            (Primitive::Date(x), Primitive::Date(y)) => {
                let result = match operator {
                    Operator::Minus => Ok(x.signed_duration_since(*y).num_seconds()),
                    _ => Err((left.type_name(), right.type_name())),
                }?;
                Ok(UntaggedValue::Primitive(Primitive::Duration(result)))
            }
            (Primitive::Date(x), Primitive::Duration(y)) => {
                let result = match operator {
                    Operator::Plus => Ok(x
                        .checked_add_signed(chrono::Duration::seconds(*y as i64))
                        .expect("Overflowing add of duration")),
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
            _ => Err((left.type_name(), right.type_name())),
        },
        _ => Err((left.type_name(), right.type_name())),
    }
}

pub fn compare_values(
    operator: Operator,
    left: &UntaggedValue,
    right: &UntaggedValue,
) -> Result<bool, (&'static str, &'static str)> {
    let coerced = coerce_compare(left, right)?;
    let ordering = coerced.compare();

    use std::cmp::Ordering;

    let result = match (operator, ordering) {
        (Operator::Equal, Ordering::Equal) => true,
        (Operator::NotEqual, Ordering::Less) | (Operator::NotEqual, Ordering::Greater) => true,
        (Operator::LessThan, Ordering::Less) => true,
        (Operator::GreaterThan, Ordering::Greater) => true,
        (Operator::GreaterThanOrEqual, Ordering::Greater)
        | (Operator::GreaterThanOrEqual, Ordering::Equal) => true,
        (Operator::LessThanOrEqual, Ordering::Less)
        | (Operator::LessThanOrEqual, Ordering::Equal) => true,
        _ => false,
    };

    Ok(result)
}

pub fn format_type<'a>(value: impl Into<&'a UntaggedValue>, width: usize) -> String {
    Type::from_value(value.into()).colored_string(width)
}

pub fn format_leaf<'a>(value: impl Into<&'a UntaggedValue>) -> DebugDocBuilder {
    InlineShape::from_value(value.into()).format().pretty()
}

pub fn style_leaf(value: &Value) -> String {
    match value {
        Value {
            value: UntaggedValue::Primitive(p),
            tag,
        } => {
            let mut styled_primitive_string = style_primitive(&p).to_string();

            if let Some(data_color) = tag.data_color {
                if let Some(data_color_string) = color_to_str(data_color) {
                    styled_primitive_string =
                        format!("F{}{}", data_color_string, style_primitive(&p));
                }
            }

            styled_primitive_string
        }
        _ => "".to_string(),
    }
}

fn color_to_str(color: color::Color) -> Option<String> {
    let color_string = match color {
        color::GREEN => "g",
        color::RED => "r",
        color::BLUE => "u",
        color::BLACK => "b",
        color::YELLOW => "y",
        color::MAGENTA => "m",
        color::CYAN => "c",
        color::WHITE => "w",
        color::BRIGHT_GREEN => "bg",
        color::BRIGHT_RED => "br",
        color::BRIGHT_BLUE => "bu",
        color::BRIGHT_YELLOW => "by",
        color::BRIGHT_MAGENTA => "bm",
        color::BRIGHT_CYAN => "bc",
        color::BRIGHT_WHITE => "bw",
        _ => return None,
    };

    Some(color_string.to_string())
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
    use super::UntaggedValue as v;
    use indexmap::indexmap;

    use super::merge_values;

    #[test]
    fn merges_tables() {
        let table_author_row = v::row(indexmap! {
            "name".into() => v::string("Andrés").into_untagged_value(),
            "country".into() => v::string("EC").into_untagged_value(),
            "date".into() => v::string("April 29-2020").into_untagged_value()
        });

        let other_table_author_row = v::row(indexmap! {
            "name".into() => v::string("YK").into_untagged_value(),
            "country".into() => v::string("US").into_untagged_value(),
            "date".into() => v::string("October 10-2019").into_untagged_value()
        });

        assert_eq!(
            other_table_author_row,
            merge_values(&table_author_row, &other_table_author_row).unwrap()
        );
    }
}
