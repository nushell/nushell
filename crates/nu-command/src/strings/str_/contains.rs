use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{
    Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str contains"
    }

    fn signature(&self) -> Signature {
        Signature::build("str contains")
            .required("pattern", SyntaxShape::String, "the pattern to find")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally check if string contains pattern by column paths",
            )
            .switch("insensitive", "search is case insensitive", Some('i'))
            .switch("not", "does not contain", Some('n'))
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Checks if string contains pattern"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["pattern", "match", "find", "search"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Check if string contains pattern",
                example: "'my_library.rb' | str contains '.rb'",
                result: Some(Value::Bool {
                    val: true,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Check if string contains pattern case insensitive",
                example: "'my_library.rb' | str contains -i '.RB'",
                result: Some(Value::Bool {
                    val: true,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Check if string contains pattern in a table",
                example: " [[ColA ColB]; [test 100]] | str contains 'e' ColA",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["ColA".to_string(), "ColB".to_string()],
                        vals: vec![
                            Value::Bool {
                                val: true,
                                span: Span::test_data(),
                            },
                            Value::test_int(100),
                        ],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Check if string contains pattern in a table",
                example: " [[ColA ColB]; [test 100]] | str contains -i 'E' ColA",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["ColA".to_string(), "ColB".to_string()],
                        vals: vec![
                            Value::Bool {
                                val: true,
                                span: Span::test_data(),
                            },
                            Value::test_int(100),
                        ],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Check if string contains pattern in a table",
                example: " [[ColA ColB]; [test hello]] | str contains 'e' ColA ColB",
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["ColA".to_string(), "ColB".to_string()],
                        vals: vec![
                            Value::Bool {
                                val: true,
                                span: Span::test_data(),
                            },
                            Value::Bool {
                                val: true,
                                span: Span::test_data(),
                            },
                        ],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Check if string contains pattern",
                example: "'hello' | str contains 'banana'",
                result: Some(Value::Bool {
                    val: false,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Check if list contains pattern",
                example: "[one two three] | str contains o",
                result: Some(Value::List {
                    vals: vec![
                        Value::Bool {
                            val: true,
                            span: Span::test_data(),
                        },
                        Value::Bool {
                            val: true,
                            span: Span::test_data(),
                        },
                        Value::Bool {
                            val: false,
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Check if list does not contain pattern",
                example: "[one two three] | str contains -n o",
                result: Some(Value::List {
                    vals: vec![
                        Value::Bool {
                            val: false,
                            span: Span::test_data(),
                        },
                        Value::Bool {
                            val: false,
                            span: Span::test_data(),
                        },
                        Value::Bool {
                            val: true,
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let pattern: Spanned<String> = call.req(engine_state, stack, 0)?;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
    let case_insensitive = call.has_flag("insensitive");
    let not_contain = call.has_flag("not");

    input.map(
        move |v| {
            if column_paths.is_empty() {
                action(&v, case_insensitive, not_contain, &pattern.item, head)
            } else {
                let mut ret = v;
                for path in &column_paths {
                    let p = pattern.item.clone();
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| action(old, case_insensitive, not_contain, &p, head)),
                    );
                    if let Err(error) = r {
                        return Value::Error { error };
                    }
                }
                ret
            }
        },
        engine_state.ctrlc.clone(),
    )
}

fn action(
    input: &Value,
    case_insensitive: bool,
    not_contain: bool,
    pattern: &str,
    head: Span,
) -> Value {
    match input {
        Value::String { val, .. } => Value::Bool {
            val: match case_insensitive {
                true => {
                    if not_contain {
                        !val.to_lowercase().contains(pattern.to_lowercase().as_str())
                    } else {
                        val.to_lowercase().contains(pattern.to_lowercase().as_str())
                    }
                }
                false => {
                    if not_contain {
                        !val.contains(pattern)
                    } else {
                        val.contains(pattern)
                    }
                }
            },
            span: head,
        },
        other => Value::Error {
            error: ShellError::UnsupportedInput(
                format!(
                    "Input's type is {}. This command only works with strings.",
                    other.get_type()
                ),
                head,
            ),
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
