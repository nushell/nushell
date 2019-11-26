use crate::data::base::property_get::get_data_by_key;
use crate::prelude::*;
use csv::WriterBuilder;
use indexmap::{indexset, IndexSet};
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, UntaggedValue, Value};
use nu_source::Spanned;

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

            return Ok(String::from_utf8(wtr.into_inner().map_err(|_| {
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
            })?);
        }
        UntaggedValue::Table(list) => {
            let mut wtr = WriterBuilder::new()
                .delimiter(separator as u8)
                .from_writer(vec![]);

            let merged_descriptors = merge_descriptors(&list);

            wtr.write_record(merged_descriptors.iter().map(|item| &item.item[..]))
                .expect("can not write.");

            for l in list {
                let mut row = vec![];
                for desc in &merged_descriptors {
                    match get_data_by_key(l, desc.borrow_spanned()) {
                        Some(s) => {
                            row.push(to_string_tagged_value(&s)?);
                        }
                        None => {
                            row.push(String::new());
                        }
                    }
                }
                wtr.write_record(&row).expect("can not write");
            }

            return Ok(String::from_utf8(wtr.into_inner().map_err(|_| {
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
            })?);
        }
        _ => return to_string_tagged_value(tagged_value),
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
            UntaggedValue::Primitive(Primitive::Boolean(b.clone()))
        }
        UntaggedValue::Primitive(Primitive::Decimal(f)) => {
            UntaggedValue::Primitive(Primitive::Decimal(f.clone()))
        }
        UntaggedValue::Primitive(Primitive::Int(i)) => {
            UntaggedValue::Primitive(Primitive::Int(i.clone()))
        }
        UntaggedValue::Primitive(Primitive::Path(x)) => {
            UntaggedValue::Primitive(Primitive::Path(x.clone()))
        }
        UntaggedValue::Primitive(Primitive::Bytes(b)) => {
            UntaggedValue::Primitive(Primitive::Bytes(b.clone()))
        }
        UntaggedValue::Primitive(Primitive::Date(d)) => {
            UntaggedValue::Primitive(Primitive::Date(d.clone()))
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
        UntaggedValue::Primitive(Primitive::Date(d)) => Ok(d.to_string()),
        UntaggedValue::Primitive(Primitive::Bytes(b)) => {
            let tmp = format!("{}", b);
            Ok(tmp)
        }
        UntaggedValue::Primitive(Primitive::Boolean(_)) => Ok(v.as_string()?.to_string()),
        UntaggedValue::Primitive(Primitive::Decimal(_)) => Ok(v.as_string()?.to_string()),
        UntaggedValue::Primitive(Primitive::Int(_)) => Ok(v.as_string()?.to_string()),
        UntaggedValue::Primitive(Primitive::Path(_)) => Ok(v.as_string()?.to_string()),
        UntaggedValue::Table(_) => return Ok(String::from("[Table]")),
        UntaggedValue::Row(_) => return Ok(String::from("[Row]")),
        UntaggedValue::Primitive(Primitive::String(s)) => return Ok(s.to_string()),
        _ => {
            return Err(ShellError::labeled_error(
                "Unexpected value",
                "",
                v.tag.clone(),
            ))
        }
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

pub fn to_delimited_data(
    headerless: bool,
    sep: char,
    format_name: &'static str,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let name_tag = name;
    let name_span = name_tag.span;

    let stream = async_stream! {
         let input: Vec<Value> = input.values.collect().await;

         let to_process_input = if input.len() > 1 {
             let tag = input[0].tag.clone();
             vec![Value { value: UntaggedValue::Table(input), tag } ]
         } else if input.len() == 1 {
             input
         } else {
             vec![]
         };

         for value in to_process_input {
             match from_value_to_delimited_string(&clone_tagged_value(&value), sep) {
                 Ok(x) => {
                     let converted = if headerless {
                         x.lines().skip(1).collect()
                     } else {
                         x
                     };
                     yield ReturnSuccess::value(UntaggedValue::Primitive(Primitive::String(converted)).into_value(&name_tag))
                 }
                 _ => {
                     let expected = format!("Expected a table with {}-compatible structure.tag() from pipeline", format_name);
                     let requires = format!("requires {}-compatible input", format_name);
                     yield Err(ShellError::labeled_error_with_secondary(
                         expected,
                         requires,
                         name_span,
                         "originates from here".to_string(),
                         value.tag.span,
                     ))
                 }
             }
         }
    };

    Ok(stream.to_output_stream())
}
