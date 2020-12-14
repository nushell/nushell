use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{PathMember, Primitive, UntaggedValue, Value};
use nu_source::{Span, TaggedItem};
use num_bigint::BigInt;

pub fn int(s: impl Into<BigInt>) -> Value {
    UntaggedValue::int(s).into_untagged_value()
}

pub fn decimal(s: BigDecimal) -> Value {
    UntaggedValue::Primitive(Primitive::Decimal(s)).into_untagged_value()
}

pub fn decimal_from_float(f: f64) -> Value {
    UntaggedValue::decimal_from_float(f, Span::unknown()).into_untagged_value()
}

pub fn string(input: impl Into<String>) -> Value {
    UntaggedValue::string(input.into()).into_untagged_value()
}

pub fn row(entries: IndexMap<String, Value>) -> Value {
    UntaggedValue::row(entries).into_untagged_value()
}

pub fn nothing() -> Value {
    UntaggedValue::nothing().into_untagged_value()
}

pub fn table(list: &[Value]) -> Value {
    UntaggedValue::table(list).into_untagged_value()
}

pub fn date(input: impl Into<String>) -> Value {
    let key = input.into().tagged_unknown();

    let date = NaiveDate::parse_from_str(key.borrow_tagged().item, "%Y-%m-%d")
        .expect("date from string failed");

    UntaggedValue::Primitive(Primitive::Date(
        DateTime::<Utc>::from_utc(date.and_hms(12, 34, 56), Utc).into(),
    ))
    .into_untagged_value()
}

pub fn column_path(paths: &str) -> Value {
    UntaggedValue::column_path(paths, Span::unknown()).into_untagged_value()
}

pub fn error_callback(
    reason: &'static str,
) -> impl FnOnce(&Value, &PathMember, ShellError) -> ShellError {
    move |_obj_source, _column_path_tried, _err| ShellError::unimplemented(reason)
}
