use crate::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value};

struct Arguments {
    substring: String,
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
        "str ends-with"
    }

    fn signature(&self) -> Signature {
        Signature::build("str ends-with")
            .input_output_types(vec![(Type::String, Type::Bool)])
            .vectorizes_over_list(true)
            .required("string", SyntaxShape::String, "the string to match")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, check strings at the given cell paths, and replace with result",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Check if an input ends with a string"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["suffix", "match", "find", "search"]
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
        };
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Checks if string ends with '.rb'",
                example: "'my_library.rb' | str ends-with '.rb'",
                result: Some(Value::Bool {
                    val: true,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Checks if string ends with '.txt'",
                example: "'my_library.rb' | str ends-with '.txt'",
                result: Some(Value::Bool {
                    val: false,
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn action(input: &Value, args: &Arguments, head: Span) -> Value {
    match input {
        Value::String { val, .. } => Value::Bool {
            val: val.ends_with(&args.substring),
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
