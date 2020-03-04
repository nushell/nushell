use crate::data::base::coerce_compare;
use crate::data::base::shape::{Column, InlineShape};
use crate::data::primitive::style_primitive;
use chrono::DateTime;
use nu_errors::ShellError;
use nu_parser::CompareOperator;
use nu_protocol::{Primitive, Type, UntaggedValue};
use nu_source::{DebugDocBuilder, PrettyDebug, Tagged};

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

pub fn compare_values(
    operator: CompareOperator,
    left: &UntaggedValue,
    right: &UntaggedValue,
) -> Result<bool, (&'static str, &'static str)> {
    match operator {
        _ => {
            let coerced = coerce_compare(left, right)?;
            let ordering = coerced.compare();

            use std::cmp::Ordering;

            let result = match (operator, ordering) {
                (CompareOperator::Equal, Ordering::Equal) => true,
                (CompareOperator::NotEqual, Ordering::Less)
                | (CompareOperator::NotEqual, Ordering::Greater) => true,
                (CompareOperator::LessThan, Ordering::Less) => true,
                (CompareOperator::GreaterThan, Ordering::Greater) => true,
                (CompareOperator::GreaterThanOrEqual, Ordering::Greater)
                | (CompareOperator::GreaterThanOrEqual, Ordering::Equal) => true,
                (CompareOperator::LessThanOrEqual, Ordering::Less)
                | (CompareOperator::LessThanOrEqual, Ordering::Equal) => true,
                _ => false,
            };

            Ok(result)
        }
    }
}

pub fn format_type<'a>(value: impl Into<&'a UntaggedValue>, width: usize) -> String {
    Type::from_value(value.into()).colored_string(width)
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
