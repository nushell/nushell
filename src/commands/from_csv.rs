use crate::object::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;
use csv::ReaderBuilder;

pub fn from_csv_string_to_value(
    s: String,
    span: impl Into<Span>,
) -> Result<Tagged<Value>, Box<dyn std::error::Error>> {
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .from_reader(s.as_bytes());
    let span = span.into();

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

            let mut row = TaggedDictBuilder::new(span);

            for (idx, entry) in row_values.iter().enumerate() {
                row.insert_tagged(
                    fields.get(idx).unwrap(),
                    Value::Primitive(Primitive::String(String::from(entry))).tagged(span),
                );
            }

            rows.push(row.into_tagged_value());
        } else {
            break;
        }
    }

    Ok(Tagged::from_item(Value::List(rows), span))
}

pub fn from_csv(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.call_info.name_span;

    Ok(out
        .values
        .map(move |a| {
            let value_span = a.span();
            match a.item {
                Value::Primitive(Primitive::String(s)) => {
                    match from_csv_string_to_value(s, value_span) {
                        Ok(x) => ReturnSuccess::value(x),
                        Err(_) => Err(ShellError::maybe_labeled_error(
                            "Could not parse as CSV",
                            "piped data failed CSV parse",
                            span,
                        )),
                    }
                }
                _ => Err(ShellError::maybe_labeled_error(
                    "Expected string values from pipeline",
                    "expects strings from pipeline",
                    span,
                )),
            }
        })
        .to_output_stream())
}
