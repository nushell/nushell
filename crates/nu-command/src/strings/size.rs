extern crate unicode_segmentation;

use std::collections::HashMap;

// use indexmap::indexmap;
use unicode_segmentation::UnicodeSegmentation;

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Example, ShellError, Signature, Span, Spanned, Type, Value};

pub struct Size;

impl Command for Size {
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
        context: &EvaluationContext,
        call: &Call,
        input: Value,
    ) -> Result<Value, ShellError> {
        size(context, call, input)
    }

    // fn examples(&self) -> Vec<Example> {
    //     vec![
    //         Example {
    //             description: "Count the number of words in a string",
    //             example: r#"echo "There are seven words in this sentence" | size"#,
    //             result: Some(vec![Value::row(indexmap! {
    //                     "lines".to_string() => UntaggedValue::int(0).into(),
    //                     "words".to_string() => UntaggedValue::int(7).into(),
    //                     "chars".to_string() => UntaggedValue::int(38).into(),
    //                     "bytes".to_string() => UntaggedValue::int(38).into(),
    //             })
    //             .into()]),
    //         },
    //         Example {
    //             description: "Counts Unicode characters correctly in a string",
    //             example: r#"echo "Amélie Amelie" | size"#,
    //             result: Some(vec![UntaggedValue::row(indexmap! {
    //                     "lines".to_string() => UntaggedValue::int(0).into(),
    //                     "words".to_string() => UntaggedValue::int(2).into(),
    //                     "chars".to_string() => UntaggedValue::int(13).into(),
    //                     "bytes".to_string() => UntaggedValue::int(15).into(),
    //             })
    //             .into()]),
    //         },
    //     ]
    // }
}

fn size(_context: &EvaluationContext, call: &Call, input: Value) -> Result<Value, ShellError> {
    let span = call.head;
    input.map(span, move |v| match v.as_string() {
        Ok(s) => count(&s, span),
        Err(_) => Value::Error {
            error: ShellError::PipelineMismatch {
                expected: Type::String,
                expected_span: span,
                origin: span,
            },
        },
    })
}

fn count(contents: &str, span: Span) -> Value {
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

    let mut item: HashMap<String, Value> = HashMap::new();
    item.insert("lines".to_string(), Value::Int { val: lines, span });
    item.insert("words".to_string(), Value::Int { val: words, span });
    item.insert("chars".to_string(), Value::Int { val: chars, span });
    item.insert("bytes".to_string(), Value::Int { val: bytes, span });

    Value::from(Spanned { item, span })
}

// #[cfg(test)]
// mod tests {
//     use super::ShellError;
//     use super::Size;

//     #[test]
//     fn examples_work_as_expected() -> Result<(), ShellError> {
//         use crate::examples::test as test_examples;

//         test_examples(Size {})
//     }
// }
