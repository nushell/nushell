use crate::prelude::*;
use csv::{ErrorKind, ReaderBuilder};
use nu_errors::ShellError;
use nu_protocol::{TaggedDictBuilder, UntaggedValue, Value};

fn from_delimited_string_to_value(
    s: String,
    noheaders: bool,
    separator: char,
    tag: impl Into<Tag>,
) -> Result<Value, csv::Error> {
    let mut reader = ReaderBuilder::new()
        .has_headers(!noheaders)
        .delimiter(separator as u8)
        .from_reader(s.as_bytes());
    let tag = tag.into();
    let span = tag.span;

    let headers = if noheaders {
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
            if let Ok(i) = value.parse::<i64>() {
                tagged_row.insert_value(header, UntaggedValue::int(i).into_value(&tag))
            } else if let Ok(f) = value.parse::<f64>() {
                tagged_row.insert_value(
                    header,
                    UntaggedValue::decimal_from_float(f, span).into_value(&tag),
                )
            } else {
                tagged_row.insert_value(header, UntaggedValue::string(value).into_value(&tag))
            }
        }
        rows.push(tagged_row.into_value());
    }

    Ok(UntaggedValue::Table(rows).into_value(&tag))
}

pub async fn from_delimited_data(
    noheaders: bool,
    sep: char,
    format_name: &'static str,
    input: InputStream,
    name: Tag,
) -> Result<OutputStream, ShellError> {
    let name_tag = name;
    let concat_string = input.collect_string(name_tag.clone()).await?;
    let sample_lines = concat_string.item.lines().take(3).collect_vec().join("\n");

    match from_delimited_string_to_value(concat_string.item, noheaders, sep, name_tag.clone()) {
        Ok(x) => match x {
            Value {
                value: UntaggedValue::Table(list),
                ..
            } => Ok(futures::stream::iter(list).to_output_stream()),
            x => Ok(OutputStream::one(x)),
        },
        Err(err) => {
            let line_one = match pretty_csv_error(err) {
                Some(pretty) => format!(
                    "Could not parse as {} split by '{}' ({})",
                    format_name, sep, pretty
                ),
                None => format!("Could not parse as {} split by '{}'", format_name, sep),
            };
            let line_two = format!(
                "input cannot be parsed as {} split by '{}'. Input's first lines:\n{}",
                format_name, sep, sample_lines
            );

            Err(ShellError::labeled_error_with_secondary(
                line_one,
                line_two,
                name_tag.clone(),
                "value originates from here",
                concat_string.tag,
            ))
        }
    }
}

fn pretty_csv_error(err: csv::Error) -> Option<String> {
    match err.kind() {
        ErrorKind::UnequalLengths {
            pos,
            expected_len,
            len,
        } => {
            if let Some(pos) = pos {
                Some(format!(
                    "Line {}: expected {} fields, found {}",
                    pos.line(),
                    expected_len,
                    len
                ))
            } else {
                Some(format!("Expected {} fields, found {}", expected_len, len))
            }
        }
        ErrorKind::Seek => Some("Internal error while parsing csv".to_string()),
        _ => None,
    }
}
