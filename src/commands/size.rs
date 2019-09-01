use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::object::{TaggedDictBuilder, Value};
use crate::prelude::*;

pub struct Size;

impl WholeStreamCommand for Size {
    fn name(&self) -> &str {
        "size"
    }

    fn signature(&self) -> Signature {
        Signature::build("size")
    }

    fn usage(&self) -> &str {
        "Gather word count statistics on the text."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        size(args, registry)
    }
}

fn size(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let input = args.input;
    let span = args.call_info.name_span;
    Ok(input
        .values
        .map(move |v| match v.item {
            Value::Primitive(Primitive::String(ref s)) => ReturnSuccess::value(count(s, v.tag())),
            _ => Err(ShellError::labeled_error_with_secondary(
                "Expected a string from pipeline",
                "requires string input",
                span,
                "value originates from here",
                v.span(),
            )),
        })
        .to_output_stream())
}

fn count(contents: &str, tag: impl Into<Tag>) -> Tagged<Value> {
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

    let mut dict = TaggedDictBuilder::new(tag);
    //TODO: add back in name when we have it in the span
    //dict.insert("name", Value::string(name));
    dict.insert("lines", Value::int(lines));
    dict.insert("words", Value::int(words));
    dict.insert("chars", Value::int(chars));
    dict.insert("max length", Value::int(bytes));

    dict.into_tagged_value()
}
