use crate::object::{Primitive, SpannedDictBuilder, Value};
use crate::prelude::*;
use csv::ReaderBuilder;

pub fn from_csv_string_to_value(
    s: String,
    span: impl Into<Span>,
) -> Result<Spanned<Value>, Box<dyn std::error::Error>> {

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

            let mut row = SpannedDictBuilder::new(span);

            for (idx, entry) in row_values.iter().enumerate() {
                row.insert_spanned(
                    fields.get(idx).unwrap(),
                    Value::Primitive(Primitive::String(String::from(entry))).spanned(span),
                );
            }

            rows.push(row.into_spanned_value());
        } else {
            break;
        }
    }

    Ok(Spanned {
        item: Value::List(rows),
        span,
    })
}

pub fn from_csv(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input;
    let span = args.call_info.name_span;

    Ok(out
        .values
        .map(move |a| match a.item {
            Value::Primitive(Primitive::String(s)) => match from_csv_string_to_value(s, span) {
                Ok(x) => ReturnSuccess::value(x.spanned(a.span)),
                Err(_) => Err(ShellError::maybe_labeled_error(
                    "Could not parse as CSV",
                    "piped data failed CSV parse",
                    span,
                )),
            },
            _ => Err(ShellError::maybe_labeled_error(
                "Expected string values from pipeline",
                "expects strings from pipeline",
                span,
            )),
        })
        .to_output_stream())
}
