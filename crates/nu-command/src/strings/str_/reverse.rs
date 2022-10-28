use crate::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value};

#[derive(Clone)]
pub struct SubCommand;

struct Arguments {
    column_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_column_paths(&mut self) -> Option<Vec<CellPath>> {
        self.column_paths.take()
    }
}

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str reverse"
    }

    fn signature(&self) -> Signature {
        Signature::build("str reverse")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally reverse text by column paths",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Reverse every string in the pipeline"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "inverse", "flip"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let column_paths = (!column_paths.is_empty()).then(|| column_paths);
        let args = Arguments { column_paths };
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Reverse a single string",
                example: "'Nushell' | str reverse",
                result: Some(Value::String {
                    val: "llehsuN".to_string(),
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Reverse multiple strings in a list",
                example: "['Nushell' 'is' 'cool'] | str reverse",
                result: Some(Value::List {
                    vals: vec![
                        Value::String {
                            val: "llehsuN".to_string(),
                            span: Span::test_data(),
                        },
                        Value::String {
                            val: "si".to_string(),
                            span: Span::test_data(),
                        },
                        Value::String {
                            val: "looc".to_string(),
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn action(input: &Value, _arg: &Arguments, head: Span) -> Value {
    match input {
        Value::String { val, .. } => Value::String {
            val: val.chars().rev().collect::<String>(),
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
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(SubCommand {})
    }
}
