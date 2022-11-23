use crate::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value};

struct Arguments {
    length: Option<i64>,
    character: Option<String>,
    cell_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_cell_paths(&mut self) -> Option<Vec<CellPath>> {
        self.cell_paths.take()
    }
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str lpad"
    }

    fn signature(&self) -> Signature {
        Signature::build("str lpad")
            .input_output_types(vec![(Type::String, Type::String)])
            .vectorizes_over_list(true)
            .required_named("length", SyntaxShape::Int, "length to pad to", Some('l'))
            .required_named(
                "character",
                SyntaxShape::String,
                "character to pad with",
                Some('c'),
            )
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, pad strings at the given cell paths",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Left-pad a string to a specific length"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["append", "truncate", "padding"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let cell_paths = (!cell_paths.is_empty()).then_some(cell_paths);
        let args = Arguments {
            length: call.get_flag(engine_state, stack, "length")?,
            character: call.get_flag(engine_state, stack, "character")?,
            cell_paths,
        };

        if args.length.expect("this exists") < 0 {
            return Err(ShellError::TypeMismatch(
                String::from("The length of the string cannot be negative"),
                call.head,
            ));
        }
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Left-pad a string with asterisks until it's 10 characters wide",
                example: "'nushell' | str lpad -l 10 -c '*'",
                result: Some(Value::string("***nushell", Span::test_data())),
            },
            Example {
                description: "Left-pad a string with zeroes until it's 10 character wide",
                example: "'123' | str lpad -l 10 -c '0'",
                result: Some(Value::string("0000000123", Span::test_data())),
            },
            Example {
                description: "Use lpad to truncate a string to its last three characters",
                example: "'123456789' | str lpad -l 3 -c '0'",
                result: Some(Value::string("789", Span::test_data())),
            },
            Example {
                description: "Use lpad to pad Unicode",
                example: "'▉' | str lpad -l 10 -c '▉'",
                result: Some(Value::string("▉▉▉▉▉▉▉▉▉▉", Span::test_data())),
            },
        ]
    }
}

fn action(
    input: &Value,
    Arguments {
        character, length, ..
    }: &Arguments,
    head: Span,
) -> Value {
    match &input {
        Value::String { val, .. } => match length {
            Some(x) => {
                let s = *x as usize;
                if s < val.len() {
                    Value::String {
                        val: val
                            .chars()
                            .rev()
                            .take(s)
                            .collect::<String>()
                            .chars()
                            .rev()
                            .collect::<String>(),
                        span: head,
                    }
                } else {
                    let c = character.as_ref().expect("we already know this flag needs to exist because the command is type checked before we call the action function");
                    let mut res = c.repeat(s - val.chars().count());
                    res += val;
                    Value::String {
                        val: res,
                        span: head,
                    }
                }
            }
            None => Value::Error {
                error: ShellError::TypeMismatch(String::from("Length argument is missing"), head),
            },
        },
        Value::Error { .. } => input.clone(),
        _ => Value::Error {
            error: ShellError::OnlySupportsThisInputType(
                "string".into(),
                input.get_type().to_string(),
                head,
                input.span().unwrap(),
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
