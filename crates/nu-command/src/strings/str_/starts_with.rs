use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::ast::CellPath;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Category;
use nu_protocol::Spanned;
use nu_protocol::{Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value};
use std::sync::Arc;

struct Arguments {
    substring: String,
    column_paths: Vec<CellPath>,
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
        operate(engine_state, stack, call, input)
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

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let substring: Spanned<String> = call.req(engine_state, stack, 0)?;

    let options = Arc::new(Arguments {
        substring: substring.item,
        column_paths: call.rest(engine_state, stack, 1)?,
    });
    let head = call.head;
    input.map(
        move |v| {
            if options.column_paths.is_empty() {
                action(&v, &options, head)
            } else {
                let mut ret = v;
                for path in &options.column_paths {
                    let opt = options.clone();
                    let r = ret.update_cell_path(
                        &path.members,
                        Box::new(move |old| action(old, &opt, head)),
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
