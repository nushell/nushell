use crate::prelude::*;
use csv::ReaderBuilder;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, TaggedDictBuilder, UntaggedValue, Value};

fn from_delimited_string_to_value(
    s: String,
    headerless: bool,
    separator: char,
    tag: impl Into<Tag>,
) -> Result<Value, csv::Error> {
    let mut reader = ReaderBuilder::new()
        .has_headers(!headerless)
        .delimiter(separator as u8)
        .from_reader(s.as_bytes());
    let tag = tag.into();

    let headers = if headerless {
        (1..=reader.headers()?.len())
            .map(|i| format!("Column{}", i))
            .collect::<Vec<String>>()
    } else {
        reader.headers()?.iter().map(String::from).collect()
    };

    let mut rows = vec![];
    for row in reader.records() {
        let mut tagged_row = TaggedDictBuilder::new(&tag);
        for (value, header) in row?.iter().zip(headers.iter()) {
            tagged_row.insert_value(
                header,
                UntaggedValue::Primitive(Primitive::String(String::from(value))).into_value(&tag),
            )
        }
        rows.push(tagged_row.into_value());
    }

    Ok(UntaggedValue::Table(rows).into_value(&tag))
}

pub fn from_delimited_data(
    headerless: bool,
    sep: char,
    format_name: &'static str,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let name_tag = name;

    let stream = async_stream! {
        let concat_string = input.collect_string(name_tag.clone()).await?;

        match from_delimited_string_to_value(concat_string.item, headerless, sep, name_tag.clone()) {
            Ok(x) => match x {
                Value { value: UntaggedValue::Table(list), .. } => {
                    for l in list {
                        yield ReturnSuccess::value(l);
                    }
                }
                x => yield ReturnSuccess::value(x),
            },
            Err(_) => {
                let line_one = format!("Could not parse as {}", format_name);
                let line_two = format!("input cannot be parsed as {}", format_name);
                yield Err(ShellError::labeled_error_with_secondary(
                    line_one,
                    line_two,
                    name_tag.clone(),
                    "value originates from here",
                    concat_string.tag,
                ))
            } ,
        }
    };

    Ok(stream.to_output_stream())
}
