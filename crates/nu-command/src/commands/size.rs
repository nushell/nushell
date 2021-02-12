extern crate unicode_segmentation;

use crate::prelude::*;
use indexmap::indexmap;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue, Value};
use unicode_segmentation::UnicodeSegmentation;

pub struct Size;

#[async_trait]
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(size(args))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Count the number of words in a string",
                example: r#"echo "There are seven words in this sentence" | size"#,
                result: Some(vec![UntaggedValue::row(indexmap! {
                        "lines".to_string() => UntaggedValue::int(0).into(),
                        "words".to_string() => UntaggedValue::int(7).into(),
                        "chars".to_string() => UntaggedValue::int(38).into(),
                        "bytes".to_string() => UntaggedValue::int(38).into(),
                })
                .into()]),
            },
            Example {
                description: "Counts unicode characters correctly in a string",
                example: r#"echo "Amélie Amelie" | size"#,
                result: Some(vec![UntaggedValue::row(indexmap! {
                        "lines".to_string() => UntaggedValue::int(0).into(),
                        "words".to_string() => UntaggedValue::int(2).into(),
                        "chars".to_string() => UntaggedValue::int(13).into(),
                        "bytes".to_string() => UntaggedValue::int(15).into(),
                })
                .into()]),
            },
        ]
    }
}

fn size(args: CommandArgs) -> OutputStream {
    let input = args.input;
    let tag = args.call_info.name_tag;
    let name_span = tag.span;

    input
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
        .to_output_stream()
}

fn count(contents: &str, tag: impl Into<Tag>) -> Value {
    let mut lines: i64 = 0;
    let mut words: i64 = 0;
    let mut chars: i64 = 0;
    let bytes = contents.len() as i64;
    let mut end_of_word = true;

    for c in UnicodeSegmentation::graphemes(contents, true) {
        chars += 1;

        match c {
            "\n" => {
                lines += 1;
                end_of_word = true;
            }
            " " => end_of_word = true,
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
    dict.insert_untagged("bytes", UntaggedValue::int(bytes));

    dict.into_value()
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::Size;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Size {})
    }
}
