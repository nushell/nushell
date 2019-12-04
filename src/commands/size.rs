use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue, Value};

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
    let tag = args.call_info.name_tag;
    let name_span = tag.span;

    Ok(input
        .values
        .map(move |v| {
            if let Ok(s) = v.as_string() {
                ReturnSuccess::value(count(&s, &v.tag))
            } else {
                Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name_span,
                    "value originates from here",
                    v.tag.span,
                ))
            }
        })
        .to_output_stream())
}

fn count(contents: &str, tag: impl Into<Tag>) -> Value {
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
    //TODO: add back in name when we have it in the tag
    //dict.insert("name", value::string(name));
    dict.insert_untagged("lines", UntaggedValue::int(lines));
    dict.insert_untagged("words", UntaggedValue::int(words));
    dict.insert_untagged("chars", UntaggedValue::int(chars));
    dict.insert_untagged("max length", UntaggedValue::int(bytes));

    dict.into_value()
}
