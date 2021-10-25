extern crate unicode_segmentation;

use unicode_segmentation::UnicodeSegmentation;

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
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
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        size(call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Count the number of words in a string",
                example: r#""There are seven words in this sentence" | size"#,
                result: Some(Value::Record {
                    cols: vec![
                        "lines".into(),
                        "words".into(),
                        "chars".into(),
                        "bytes".into(),
                    ],
                    vals: vec![
                        Value::Int {
                            val: 0,
                            span: Span::unknown(),
                        },
                        Value::Int {
                            val: 7,
                            span: Span::unknown(),
                        },
                        Value::Int {
                            val: 38,
                            span: Span::unknown(),
                        },
                        Value::Int {
                            val: 38,
                            span: Span::unknown(),
                        },
                    ],
                    span: Span::unknown(),
                }),
            },
            Example {
                description: "Counts Unicode characters correctly in a string",
                example: r#""Amélie Amelie" | size"#,
                result: Some(Value::Record {
                    cols: vec![
                        "lines".into(),
                        "words".into(),
                        "chars".into(),
                        "bytes".into(),
                    ],
                    vals: vec![
                        Value::Int {
                            val: 0,
                            span: Span::unknown(),
                        },
                        Value::Int {
                            val: 2,
                            span: Span::unknown(),
                        },
                        Value::Int {
                            val: 13,
                            span: Span::unknown(),
                        },
                        Value::Int {
                            val: 15,
                            span: Span::unknown(),
                        },
                    ],
                    span: Span::unknown(),
                }),
            },
        ]
    }
}

fn size(call: &Call, input: PipelineData) -> Result<PipelineData, ShellError> {
    let span = call.head;
    Ok(input
        .map(move |v| match v.as_string() {
            Ok(s) => count(&s, span),
            Err(_) => Value::Error {
                error: ShellError::PipelineMismatch {
                    expected: Type::String,
                    expected_span: span,
                    origin: span,
                },
            },
        })
        .into_pipeline_data())
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

    let mut cols = vec![];
    let mut vals = vec![];

    cols.push("lines".into());
    vals.push(Value::Int { val: lines, span });

    cols.push("words".into());
    vals.push(Value::Int { val: words, span });

    cols.push("chars".into());
    vals.push(Value::Int { val: chars, span });

    cols.push("bytes".into());
    vals.push(Value::Int { val: bytes, span });

    Value::Record { cols, vals, span }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Size {})
    }
}
