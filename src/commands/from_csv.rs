use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;
use csv::ReaderBuilder;

pub fn from_csv_string_to_value(
    s: String,
    tag: impl Into<Tag>,
) -> Result<Tagged<Value>, Box<dyn std::error::Error>> {
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .from_reader(s.as_bytes());
    let tag = tag.into();

    let mut fields: VecDeque<String> = VecDeque::new();
    let mut iter = reader.records();
    let mut rows = vec![];

    if let Some(result) = iter.next() {
        let line = result?;

        for item in line.iter() {
            fields.push_back(item.to_string());
        }
    }

    loop {
        if let Some(row_values) = iter.next() {
            let row_values = row_values?;

            let mut row = TaggedDictBuilder::new(tag);

            for (idx, entry) in row_values.iter().enumerate() {
                row.insert_tagged(
                    fields.get(idx).unwrap(),
                    Value::Primitive(Primitive::String(String::from(entry))).tagged(tag),
                );
            }

            rows.push(row.into_tagged_value());
        } else {
            break;
        }
    }

    Ok(Tagged::from_item(Value::List(rows), tag))
}

pub fn from_csv(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let span = args.name_span();
    let out = args.input;

    Ok(out
        .values
        .map(move |a| {
            let value_tag = a.tag();
            match a.item {
                Value::Primitive(Primitive::String(s)) => {
                    match from_csv_string_to_value(s, value_tag) {
                        Ok(x) => ReturnSuccess::value(x),
                        Err(_) => Err(ShellError::labeled_error_with_secondary(
                            "Could not parse as CSV",
                            "input cannot be parsed as CSV",
                            span,
                            "value originates from here",
                            value_tag.span,
                        )),
                    }
                }
                _ => Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    span,
                    "value originates from here",
                    a.span(),
                )),
            }
        })
        .to_output_stream())
}
