use crate::input_handler::{operate, CellPathOnlyArgs};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value};
#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str length"
    }

    fn signature(&self) -> Signature {
        Signature::build("str length")
            .input_output_types(vec![(Type::String, Type::Int)])
            .vectorizes_over_list(true)
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "For a data structure input, replace strings at the given cell paths with their length",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Output the length of any strings in the pipeline"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["size", "count"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let args = CellPathOnlyArgs::from(cell_paths);
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return the lengths of multiple strings",
                example: "'hello' | str length",
                result: Some(Value::test_int(5)),
            },
            Example {
                description: "Return the lengths of multiple strings",
                example: "['hi' 'there'] | str length",
                result: Some(Value::List {
                    vals: vec![Value::test_int(2), Value::test_int(5)],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn action(input: &Value, _arg: &CellPathOnlyArgs, head: Span) -> Value {
    match input {
        Value::String { val, .. } => Value::Int {
            val: val.len() as i64,
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
