use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, ListStream, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct Echo;

impl Command for Echo {
    fn name(&self) -> &str {
        "echo"
    }

    fn usage(&self) -> &str {
        "Echo the arguments back to the user."
    }

    fn signature(&self) -> Signature {
        Signature::build("echo")
            .rest("rest", SyntaxShape::Any, "the values to echo")
            .switch("no-newline", "Remove trailing newline", Some('n'))
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        "Unlike `print`, this command returns an actual value that will be passed to the next command of the pipeline."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let no_newline = call.has_flag("no-newline");
        call.rest(engine_state, stack, 0).map(|to_be_echoed| {
            let n = to_be_echoed.len();
            match n.cmp(&1usize) {
                //  More than one value is converted in a stream of values
                std::cmp::Ordering::Greater => PipelineData::ListStream(
                    ListStream::from_stream(
                        to_be_echoed
                            .into_iter()
                            .map(move |it| remove_newline(it, no_newline)),
                        engine_state.ctrlc.clone(),
                    ),
                    None,
                ),

                //  But a single value can be forwarded as it is
                std::cmp::Ordering::Equal => {
                    let value = remove_newline(to_be_echoed[0].clone(), no_newline);

                    PipelineData::Value(value, None)
                }

                //  When there are no elements, we echo the empty string
                std::cmp::Ordering::Less => PipelineData::Value(
                    Value::String {
                        val: "".to_string(),
                        span: call.head,
                    },
                    None,
                ),
            }
        })
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Put a hello message in the pipeline",
                example: "echo 'hello'",
                result: Some(Value::test_string("hello")),
            },
            Example {
                description: "Print the value of the special '$nu' variable",
                example: "echo $nu",
                result: None,
            },
            Example {
                description: "Remove trailing newline from a single string",
                example: "echo -n 'hello\n\n'",
                result: Some(Value::test_string("hello\n")),
            },
            Example {
                description: "Remove trailing newline from multi strings",
                example: "echo -n 'hello\n' 'hello\n\n'",
                result: Some(Value::List {
                    vals: vec![Value::test_string("hello"), Value::test_string("hello\n")],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Remove trailing newline from a string list",
                example: "echo -n ['hello\n', 'hello\n\n', 'hello\n\n\n']",
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("hello"),
                        Value::test_string("hello\n"),
                        Value::test_string("hello\n\n"),
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn remove_newline(value: Value, no_newline: bool) -> Value {
    if !no_newline {
        return value;
    }

    match value {
        Value::List { vals, span } if vals.iter().all(|it| it.get_type() == Type::String) => {
            let vals = vals
                .into_iter()
                .map(|it| {
                    Value::string(
                        strip_suffix(unsafe { it.as_string().unwrap_unchecked() }, "\n"),
                        unsafe { it.span().unwrap_unchecked() },
                    )
                })
                .collect();

            Value::List { vals, span }
        }
        Value::String { val, span } => Value::string(strip_suffix(val, "\n"), span),
        _ => value,
    }
}

#[inline]
fn strip_suffix(s: String, suffix: &str) -> String {
    match s.strip_suffix(suffix) {
        Some(s) => s.to_string(),
        _ => s,
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Echo;
        use crate::test_examples;
        test_examples(Echo {})
    }
}
