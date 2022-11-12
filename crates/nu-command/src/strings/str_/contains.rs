use crate::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

struct Arguments {
    substring: String,
    cell_paths: Option<Vec<CellPath>>,
    case_insensitive: bool,
    not_contain: bool,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str contains"
    }

    fn signature(&self) -> Signature {
        Signature::build("str contains")
            .input_output_types(vec![(Type::String, Type::Bool)])
            .vectorizes_over_list(true)
            .required("string", SyntaxShape::String, "the substring to find")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, check strings at the given cell paths, and replace with result",
            )
            .switch("insensitive", "search is case insensitive", Some('i'))
            .switch("not", "does not contain", Some('n'))
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Checks if string input contains a substring"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["substring", "match", "find", "search"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            substring: call.req::<String>(engine_state, stack, 0)?,
            cell_paths,
            case_insensitive: call.has_flag("insensitive"),
            not_contain: call.has_flag("not"),
        };
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Check if input contains string",
                example: "'my_library.rb' | str contains '.rb'",
                result: Some(Value::Bool {
                    val: true,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Check if input contains string case insensitive",
                example: "'my_library.rb' | str contains -i '.RB'",
                result: Some(Value::Bool {
                    val: true,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Check if input contains string in a table",
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
                description: "Check if input contains string in a table",
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
                description: "Check if input contains string in a table",
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
                description: "Check if input string contains 'banana'",
                example: "'hello' | str contains 'banana'",
                result: Some(Value::Bool {
                    val: false,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Check if list contains string",
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
                description: "Check if list does not contain string",
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

fn action(
    input: &Value,
    Arguments {
        case_insensitive,
        not_contain,
        substring,
        ..
    }: &Arguments,
    head: Span,
) -> Value {
    match input {
        Value::String { val, .. } => Value::Bool {
            val: match case_insensitive {
                true => {
                    if *not_contain {
                        !val.to_lowercase()
                            .contains(substring.to_lowercase().as_str())
                    } else {
                        val.to_lowercase()
                            .contains(substring.to_lowercase().as_str())
                    }
                }
                false => {
                    if *not_contain {
                        !val.contains(substring)
                    } else {
                        val.contains(substring)
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
