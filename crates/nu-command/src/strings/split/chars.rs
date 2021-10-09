use nu_protocol::{
    ast::Call,
    engine::{Command, EvaluationContext},
    Example, IntoValueStream, ShellError, Signature, Span, Type, Value,
};

pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "split chars"
    }

    fn signature(&self) -> Signature {
        Signature::build("split chars")
    }

    fn usage(&self) -> &str {
        "splits a string's characters into separate rows"
    }

    fn run(
        &self,
        _context: &EvaluationContext,
        call: &Call,
        input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        split_chars(call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Split the string's characters into separate rows",
            example: "echo 'hello' | split chars",
            result: Some(vec![
                Value::String {
                    val: "h".into(),
                    span: Span::unknown(),
                },
                Value::String {
                    val: "e".into(),
                    span: Span::unknown(),
                },
                Value::String {
                    val: "l".into(),
                    span: Span::unknown(),
                },
                Value::String {
                    val: "l".into(),
                    span: Span::unknown(),
                },
                Value::String {
                    val: "o".into(),
                    span: Span::unknown(),
                },
            ]),
        }]
    }
}

fn split_chars(call: &Call, input: Value) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
    let name = call.head;

    Ok(match input {
        Value::List { vals, span } => Value::List {
            vals: vals
                .iter()
                .flat_map(move |v| {
                    if let Ok(s) = v.as_string() {
                        let v_span = v.span();
                        s.chars()
                            .collect::<Vec<_>>()
                            .into_iter()
                            .map(move |x| Value::String {
                                val: x.to_string(),
                                span: v_span,
                            })
                            .collect()
                    } else {
                        vec![Value::Error {
                            error: ShellError::PipelineMismatch {
                                expected: Type::String,
                                expected_span: name,
                                origin: v.span(),
                            },
                        }]
                    }
                })
                .collect(),
            span,
        },
        Value::Stream { stream, span } => Value::Stream {
            stream: stream
                .flat_map(move |v| {
                    if let Ok(s) = v.as_string() {
                        let v_span = v.span();
                        s.chars()
                            .collect::<Vec<_>>()
                            .into_iter()
                            .map(move |x| Value::String {
                                val: x.to_string(),
                                span: v_span,
                            })
                            .collect()
                    } else {
                        vec![Value::Error {
                            error: ShellError::PipelineMismatch {
                                expected: Type::String,
                                expected_span: name,
                                origin: v.span(),
                            },
                        }]
                    }
                })
                .into_value_stream(),
            span,
        },
        v => {
            let v_span = v.span();
            if let Ok(s) = v.as_string() {
                Value::List {
                    vals: s
                        .chars()
                        .collect::<Vec<_>>()
                        .into_iter()
                        .map(move |x| Value::String {
                            val: x.to_string(),
                            span: v_span,
                        })
                        .collect(),
                    span: v_span,
                }
            } else {
                Value::Error {
                    error: ShellError::PipelineMismatch {
                        expected: Type::String,
                        expected_span: name,
                        origin: v.span(),
                    },
                }
            }
        }
    })
}

// #[cfg(test)]
// mod tests {
//     use super::ShellError;
//     use super::SubCommand;

//     #[test]
//     fn examples_work_as_expected() -> Result<(), ShellError> {
//         use crate::examples::test as test_examples;

//         test_examples(SubCommand {})
//     }
// }
