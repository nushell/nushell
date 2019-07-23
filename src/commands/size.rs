use crate::errors::ShellError;
use crate::object::{SpannedDictBuilder, Value};
use crate::prelude::*;

pub fn size(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let input = args.input;
    Ok(input
        .values
        .map(move |v| match v.item {
            Value::Primitive(Primitive::String(s)) => ReturnSuccess::value(count(&s, v.span)),
            _ => Err(ShellError::maybe_labeled_error(
                "Expected string values from pipeline",
                "expects strings from pipeline",
                Some(v.span),
            )),
        })
        .to_output_stream())
}

fn count(contents: &str, span: impl Into<Span>) -> Spanned<Value> {
    let mut lines: i64 = 0;
    let mut words: i64 = 0;
    let mut chars: i64 = 0;
    let bytes = contents.len() as i64;
    let mut end_of_word = true;

    for c in contents.chars() {
        chars += 1;

        match c {
            '\n' => {
                lines += 1;
                end_of_word = true;
            }
            ' ' => end_of_word = true,
            _ => {
                if end_of_word {
                    words += 1;
                }
                end_of_word = false;
            }
        }
    }

    let mut dict = SpannedDictBuilder::new(span);
    //TODO: add back in name when we have it in the span
    //dict.insert("name", Value::string(name));
    dict.insert("lines", Value::int(lines));
    dict.insert("words", Value::int(words));
    dict.insert("chars", Value::int(chars));
    dict.insert("max length", Value::int(bytes));

    dict.into_spanned_value()
}
