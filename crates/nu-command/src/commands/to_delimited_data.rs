use crate::prelude::*;
use csv::WriterBuilder;
use indexmap::{indexset, IndexSet};
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, UntaggedValue, Value};
use nu_source::Spanned;
use nu_value_ext::{as_string, ValueExt};

fn from_value_to_delimited_string(
    tagged_value: &Value,
    separator: char,
) -> Result<String, ShellError> {
    let v = &tagged_value.value;

    match v {
        UntaggedValue::Row(o) => {
            let mut wtr = WriterBuilder::new()
                .delimiter(separator as u8)
                .from_writer(vec![]);
            let mut fields: VecDeque<String> = VecDeque::new();
            let mut values: VecDeque<String> = VecDeque::new();

            for (k, v) in o.entries.iter() {
                fields.push_back(k.clone());

                values.push_back(to_string_tagged_value(&v)?);
            }

            wtr.write_record(fields).expect("can not write.");
            wtr.write_record(values).expect("can not write.");

            let v = String::from_utf8(wtr.into_inner().map_err(|_| {
                ShellError::labeled_error(
                    "Could not convert record",
                    "original value",
                    &tagged_value.tag,
                )
            })?)
            .map_err(|_| {
                ShellError::labeled_error(
                    "Could not convert record",
                    "original value",
                    &tagged_value.tag,
                )
            })?;
            Ok(v)
        }
        UntaggedValue::Table(list) => {
            let mut wtr = WriterBuilder::new()
                .delimiter(separator as u8)
                .from_writer(vec![]);

            let merged_descriptors = merge_descriptors(&list);

            if merged_descriptors.is_empty() {
                wtr.write_record(
                    list.iter()
                        .map(|ele| to_string_tagged_value(ele).unwrap_or_else(|_| String::new()))
                        .collect::<Vec<_>>(),
                )
                .expect("can not write");
            } else {
                wtr.write_record(merged_descriptors.iter().map(|item| &item.item[..]))
                    .expect("can not write.");

                for l in list {
                    let mut row = vec![];
                    for desc in &merged_descriptors {
                        row.push(match l.get_data_by_key(desc.borrow_spanned()) {
                            Some(s) => to_string_tagged_value(&s)?,
                            None => String::new(),
                        });
                    }
                    wtr.write_record(&row).expect("can not write");
                }
            }
            let v = String::from_utf8(wtr.into_inner().map_err(|_| {
                ShellError::labeled_error(
                    "Could not convert record",
                    "original value",
                    &tagged_value.tag,
                )
            })?)
            .map_err(|_| {
                ShellError::labeled_error(
                    "Could not convert record",
                    "original value",
                    &tagged_value.tag,
                )
            })?;
            Ok(v)
        }
        _ => to_string_tagged_value(tagged_value),
    }
}

// NOTE: could this be useful more widely and implemented on Value ?
pub fn clone_tagged_value(v: &Value) -> Value {
    match &v.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            UntaggedValue::Primitive(Primitive::String(s.clone()))
        }
        UntaggedValue::Primitive(Primitive::Nothing) => {
            UntaggedValue::Primitive(Primitive::Nothing)
        }
        UntaggedValue::Primitive(Primitive::Boolean(b)) => {
            UntaggedValue::Primitive(Primitive::Boolean(*b))
        }
        UntaggedValue::Primitive(Primitive::Decimal(f)) => {
            UntaggedValue::Primitive(Primitive::Decimal(f.clone()))
        }
        UntaggedValue::Primitive(Primitive::Int(i)) => {
            UntaggedValue::Primitive(Primitive::Int(i.clone()))
        }
        UntaggedValue::Primitive(Primitive::FilePath(x)) => {
            UntaggedValue::Primitive(Primitive::FilePath(x.clone()))
        }
        UntaggedValue::Primitive(Primitive::Filesize(b)) => {
            UntaggedValue::Primitive(Primitive::Filesize(b.clone()))
        }
        UntaggedValue::Primitive(Primitive::Date(d)) => {
            UntaggedValue::Primitive(Primitive::Date(*d))
        }
        UntaggedValue::Row(o) => UntaggedValue::Row(o.clone()),
        UntaggedValue::Table(l) => UntaggedValue::Table(l.clone()),
        UntaggedValue::Block(_) => UntaggedValue::Primitive(Primitive::Nothing),
        _ => UntaggedValue::Primitive(Primitive::Nothing),
    }
    .into_value(v.tag.clone())
}

// NOTE: could this be useful more widely and implemented on Value ?
fn to_string_tagged_value(v: &Value) -> Result<String, ShellError> {
    match &v.value {
        UntaggedValue::Primitive(Primitive::String(_))
        | UntaggedValue::Primitive(Primitive::Filesize(_))
        | UntaggedValue::Primitive(Primitive::Boolean(_))
        | UntaggedValue::Primitive(Primitive::Decimal(_))
        | UntaggedValue::Primitive(Primitive::FilePath(_))
        | UntaggedValue::Primitive(Primitive::Int(_)) => as_string(v),
        UntaggedValue::Primitive(Primitive::Date(d)) => Ok(d.to_string()),
        UntaggedValue::Primitive(Primitive::Nothing) => Ok(String::new()),
        UntaggedValue::Table(_) => Ok(String::from("[Table]")),
        UntaggedValue::Row(_) => Ok(String::from("[Row]")),
        _ => Err(ShellError::labeled_error(
            "Unexpected value",
            "",
            v.tag.clone(),
        )),
    }
}

fn merge_descriptors(values: &[Value]) -> Vec<Spanned<String>> {
    let mut ret: Vec<Spanned<String>> = vec![];
    let mut seen: IndexSet<String> = indexset! {};
    for value in values {
        for desc in value.data_descriptors() {
            if !seen.contains(&desc[..]) {
                seen.insert(desc.clone());
                ret.push(desc.spanned(value.tag.span));
            }
        }
    }
    ret
}

pub async fn to_delimited_data(
    headerless: bool,
    sep: char,
    format_name: &'static str,
    input: InputStream,
    name: Tag,
) -> Result<OutputStream, ShellError> {
    let name_tag = name;
    let name_span = name_tag.span;

    let input: Vec<Value> = input.collect().await;

    let to_process_input = match input.len() {
        x if x > 1 => {
            let tag = input[0].tag.clone();
            vec![Value {
                value: UntaggedValue::Table(input),
                tag,
            }]
        }
        1 => input,
        _ => vec![],
    };

    Ok(
        futures::stream::iter(to_process_input.into_iter().map(move |value| {
            match from_value_to_delimited_string(&clone_tagged_value(&value), sep) {
                Ok(mut x) => {
                    if headerless {
                        if let Some(second_line) = x.find('\n') {
                            let start = second_line + 1;
                            x.replace_range(0..start, "");
                        }
                    }
                    ReturnSuccess::value(
                        UntaggedValue::Primitive(Primitive::String(x)).into_value(&name_tag),
                    )
                }
                Err(_) => {
                    let expected = format!(
                        "Expected a table with {}-compatible structure from pipeline",
                        format_name
                    );
                    let requires = format!("requires {}-compatible input", format_name);
                    Err(ShellError::labeled_error_with_secondary(
                        expected,
                        requires,
                        name_span,
                        "originates from here".to_string(),
                        value.tag.span,
                    ))
                }
            }
        }))
        .to_output_stream(),
    )
}
