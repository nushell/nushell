use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str upcase"
    }

    fn signature(&self) -> Signature {
        Signature::build("str upcase")
            .input_output_types(vec![(Type::String, Type::String)])
            .vectorizes_over_list(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, convert strings at the given cell paths",
            )
    }

    fn usage(&self) -> &str {
        "Make text uppercase"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["uppercase", "upper case"]
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
        vec![Example {
            description: "Upcase contents",
            example: "'nu' | str upcase",
            result: Some(Value::test_string("NU")),
        }]
    }
}

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    input.map(
        move |v| {
            if column_paths.is_empty() {
                action(&v, head)
            } else {
                let mut ret = v;
                for path in &column_paths {
                    let r =
                        ret.update_cell_path(&path.members, Box::new(move |old| action(old, head)));
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

fn action(input: &Value, head: Span) -> Value {
    match input {
        Value::String { val: s, .. } => Value::String {
            val: s.to_uppercase(),
            span: head,
        },
        other => {
            let got = format!("Expected string but got {}", other.get_type());
            Value::Error {
                error: ShellError::UnsupportedInput(got, head),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{action, SubCommand};

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn upcases() {
        let word = Value::test_string("andres");

        let actual = action(&word, Span::test_data());
        let expected = Value::test_string("ANDRES");
        assert_eq!(actual, expected);
    }
}
