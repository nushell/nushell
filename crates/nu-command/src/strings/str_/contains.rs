use nu_cmd_base::input_handler::{operate, CmdArgument};
use nu_engine::command_prelude::*;

use nu_utils::IgnoreCaseExt;

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
            .input_output_types(vec![
                (Type::String, Type::Bool),
                // TODO figure out cell-path type behavior
                (Type::table(), Type::table()),
                (Type::record(), Type::record()),
                (Type::List(Box::new(Type::String)), Type::List(Box::new(Type::Bool)))
            ])
            .required("string", SyntaxShape::String, "The substring to find.")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, check strings at the given cell paths, and replace with result.",
            )
            .switch("ignore-case", "search is case insensitive", Some('i'))
            .switch("not", "does not contain", Some('n'))
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Checks if string input contains a substring."
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
            case_insensitive: call.has_flag(engine_state, stack, "ignore-case")?,
            not_contain: call.has_flag(engine_state, stack, "not")?,
        };
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Check if input contains string",
                example: "'my_library.rb' | str contains '.rb'",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Check if input contains string case insensitive",
                example: "'my_library.rb' | str contains --ignore-case '.RB'",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Check if input contains string in a record",
                example: "{ ColA: test, ColB: 100 } | str contains 'e' ColA",
                result: Some(Value::test_record(record! {
                    "ColA" => Value::test_bool(true),
                    "ColB" => Value::test_int(100),
                })),
            },
            Example {
                description: "Check if input contains string in a table",
                example: " [[ColA ColB]; [test 100]] | str contains --ignore-case 'E' ColA",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "ColA" => Value::test_bool(true),
                    "ColB" => Value::test_int(100),
                })])),
            },
            Example {
                description: "Check if input contains string in a table",
                example: " [[ColA ColB]; [test hello]] | str contains 'e' ColA ColB",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "ColA" => Value::test_bool(true),
                    "ColB" => Value::test_bool(true),
                })])),
            },
            Example {
                description: "Check if input string contains 'banana'",
                example: "'hello' | str contains 'banana'",
                result: Some(Value::test_bool(false)),
            },
            Example {
                description: "Check if list contains string",
                example: "[one two three] | str contains o",
                result: Some(Value::test_list(vec![
                    Value::test_bool(true),
                    Value::test_bool(true),
                    Value::test_bool(false),
                ])),
            },
            Example {
                description: "Check if list does not contain string",
                example: "[one two three] | str contains --not o",
                result: Some(Value::test_list(vec![
                    Value::test_bool(false),
                    Value::test_bool(false),
                    Value::test_bool(true),
                ])),
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
        Value::String { val, .. } => Value::bool(
            match case_insensitive {
                true => {
                    if *not_contain {
                        !val.to_folded_case()
                            .contains(substring.to_folded_case().as_str())
                    } else {
                        val.to_folded_case()
                            .contains(substring.to_folded_case().as_str())
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
            head,
        ),
        Value::Error { .. } => input.clone(),
        _ => Value::error(
            ShellError::OnlySupportsThisInputType {
                exp_input_type: "string".into(),
                wrong_type: input.get_type().to_string(),
                dst_span: head,
                src_span: input.span(),
            },
            head,
        ),
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
