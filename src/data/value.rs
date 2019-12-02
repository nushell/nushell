use crate::data::base::coerce_compare;
use crate::data::base::shape::{Column, InlineShape, TypeShape};
use crate::data::primitive::style_primitive;
use crate::data::value;
use bigdecimal::BigDecimal;
use chrono::DateTime;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_parser::Number;
use nu_parser::Operator;
use nu_protocol::{ColumnPath, PathMember, Primitive, UntaggedValue, Value};
use nu_source::{DebugDocBuilder, PrettyDebug, Tagged};
use num_bigint::BigInt;
use std::path::PathBuf;
use std::time::SystemTime;

#[allow(unused)]
pub fn row(entries: IndexMap<String, Value>) -> UntaggedValue {
    UntaggedValue::Row(entries.into())
}

pub fn table(list: &Vec<Value>) -> UntaggedValue {
    UntaggedValue::Table(list.to_vec())
}

pub fn string(s: impl Into<String>) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::String(s.into()))
}

pub fn column_path(s: Vec<impl Into<PathMember>>) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::ColumnPath(ColumnPath::new(
        s.into_iter().map(|p| p.into()).collect(),
    )))
}

pub fn int(i: impl Into<BigInt>) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::Int(i.into()))
}

pub fn pattern(s: impl Into<String>) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::String(s.into()))
}

pub fn path(s: impl Into<PathBuf>) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::Path(s.into()))
}

pub fn bytes(s: impl Into<u64>) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::Bytes(s.into()))
}

pub fn decimal(s: impl Into<BigDecimal>) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::Decimal(s.into()))
}

pub fn binary(binary: Vec<u8>) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::Binary(binary))
}

pub fn number(s: impl Into<Number>) -> UntaggedValue {
    let num = s.into();

    match num {
        Number::Int(int) => value::int(int),
        Number::Decimal(decimal) => value::decimal(decimal),
    }
}

pub fn boolean(s: impl Into<bool>) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::Boolean(s.into()))
}

pub fn duration(secs: u64) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::Duration(secs))
}

pub fn system_date(s: SystemTime) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::Date(s.into()))
}

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

pub fn nothing() -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::Nothing)
}

pub fn compare_values(
    operator: &Operator,
    left: &UntaggedValue,
    right: &UntaggedValue,
) -> Result<bool, (&'static str, &'static str)> {
    match operator {
        _ => {
            let coerced = coerce_compare(left, right)?;
            let ordering = coerced.compare();

            use std::cmp::Ordering;

            let result = match (operator, ordering) {
                (Operator::Equal, Ordering::Equal) => true,
                (Operator::NotEqual, Ordering::Less) | (Operator::NotEqual, Ordering::Greater) => {
                    true
                }
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
    }
}

pub fn format_type<'a>(value: impl Into<&'a UntaggedValue>, width: usize) -> String {
    TypeShape::from_value(value.into()).colored_string(width)
}

pub fn format_leaf<'a>(value: impl Into<&'a UntaggedValue>) -> DebugDocBuilder {
    InlineShape::from_value(value.into()).format().pretty()
}

pub fn style_leaf<'a>(value: impl Into<&'a UntaggedValue>) -> &'static str {
    match value.into() {
        UntaggedValue::Primitive(p) => style_primitive(p),
        _ => "",
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
