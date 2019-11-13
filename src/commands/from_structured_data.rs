use crate::data::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;
use csv::ReaderBuilder;

fn from_stuctured_string_to_value(
    s: String,
    headerless: bool,
    separator: char,
    tag: impl Into<Tag>,
) -> Result<Tagged<Value>, csv::Error> {
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
            tagged_row.insert_tagged(
                header,
                Value::Primitive(Primitive::String(String::from(value))).tagged(&tag),
            )
        }
        rows.push(tagged_row.into_tagged_value());
    }

    Ok(Value::Table(rows).tagged(&tag))
}

pub fn from_structured_data(
    headerless: bool,
    sep: char,
    format_name: &'static str,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let name_tag = name;

    let stream = async_stream! {
        let values: Vec<Tagged<Value>> = input.values.collect().await;

        let mut concat_string = String::new();
        let mut latest_tag: Option<Tag> = None;

        for value in values {
            let value_tag = value.tag();
            latest_tag = Some(value_tag.clone());
            match value.item {
                Value::Primitive(Primitive::String(s)) => {
                    concat_string.push_str(&s);
                    concat_string.push_str("\n");
                }
                _ => yield Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name_tag.clone(),
                    "value originates from here",
                    value_tag.clone(),
                )),

            }
        }

        match from_stuctured_string_to_value(concat_string, headerless, sep, name_tag.clone()) {
            Ok(x) => match x {
                Tagged { item: Value::Table(list), .. } => {
                    for l in list {
                        yield ReturnSuccess::value(l);
                    }
                }
                x => yield ReturnSuccess::value(x),
            },
            Err(_) => if let Some(last_tag) = latest_tag {
                let line_one = format!("Could not parse as {}", format_name);
                let line_two = format!("input cannot be parsed as {}", format_name);
                yield Err(ShellError::labeled_error_with_secondary(
                    line_one,
                    line_two,
                    name_tag.clone(),
                    "value originates from here",
                    last_tag.clone(),
                ))
            } ,
        }
    };

    Ok(stream.to_output_stream())
}
