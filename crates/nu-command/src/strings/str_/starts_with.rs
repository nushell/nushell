use crate::input_handler::{operate, CmdArgument};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::Spanned;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value};

struct Arguments {
    substring: String,
    column_paths: Option<Vec<CellPath>>,
}

impl CmdArgument for Arguments {
    fn take_column_paths(&mut self) -> Option<Vec<CellPath>> {
        self.column_paths.take()
    }
}

#[derive(Clone)]

pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "str starts-with"
    }

    fn signature(&self) -> Signature {
        Signature::build("str starts-with")
            .required("string", SyntaxShape::String, "the string to match")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "optionally matches prefix of text by column paths",
            )
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Check if an input starts with a string"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["prefix", "match", "find", "search"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let substring: Spanned<String> = call.req(engine_state, stack, 0)?;
        let column_paths: Vec<CellPath> = call.rest(engine_state, stack, 1)?;
        let column_paths = (!column_paths.is_empty()).then(|| column_paths);
        let args = Arguments {
            substring: substring.item,
            column_paths,
        };
        operate(action, args, input, call.head, engine_state.ctrlc.clone())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Checks if input string starts with 'my'",
                example: "'my_library.rb' | str starts-with 'my'",
                result: Some(Value::Bool {
                    val: true,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Checks if input string starts with 'my'",
                example: "'Cargo.toml' | str starts-with 'Car'",
                result: Some(Value::Bool {
                    val: true,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Checks if input string starts with 'my'",
                example: "'Cargo.toml' | str starts-with '.toml'",
                result: Some(Value::Bool {
                    val: false,
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn action(input: &Value, Arguments { substring, .. }: &Arguments, head: Span) -> Value {
    match input {
        Value::String { val: s, .. } => {
            let starts_with = s.starts_with(substring);
            Value::Bool {
                val: starts_with,
                span: head,
            }
        }
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
