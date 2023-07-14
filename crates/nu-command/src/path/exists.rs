use std::path::{Path, PathBuf};

use nu_engine::{current_dir, CallExt};
use nu_path::expand_path_with;
use nu_protocol::ast::Call;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{
    engine::Command, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

use super::PathSubcommandArguments;

struct Arguments {
    columns: Option<Vec<String>>,
    pwd: PathBuf,
}

impl PathSubcommandArguments for Arguments {
    fn get_columns(&self) -> Option<Vec<String>> {
        self.columns.clone()
    }
}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "path exists"
    }

    fn signature(&self) -> Signature {
        Signature::build("path exists")
            .input_output_types(vec![
                (Type::String, Type::Bool),
                (
                    Type::List(Box::new(Type::Bool)),
                    Type::List(Box::new(Type::Bool)),
                ),
            ])
            .named(
                "columns",
                SyntaxShape::Table(vec![]),
                "For a record or table input, check strings at the given columns, and replace with result",
                Some('c'),
            )
    }

    fn usage(&self) -> &str {
        "Check whether a path exists."
    }

    fn extra_usage(&self) -> &str {
        r#"This only checks if it is possible to either `open` or `cd` to the given path.
If you need to distinguish dirs and files, please use `path type`."#
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let args = Arguments {
            columns: call.get_flag(engine_state, stack, "columns")?,
            pwd: current_dir(engine_state, stack)?,
        };
        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&exists, &args, value, head),
            engine_state.ctrlc.clone(),
        )
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Check if a file exists",
                example: "'C:\\Users\\joe\\todo.txt' | path exists",
                result: Some(Value::test_bool(false)),
            },
            Example {
                description: "Check if a file exists in a column",
                example: "ls | path exists -c [ name ]",
                result: None,
            },
        ]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Check if a file exists",
                example: "'/home/joe/todo.txt' | path exists",
                result: Some(Value::test_bool(false)),
            },
            Example {
                description: "Check if a file exists in a column",
                example: "ls | path exists -c [ name ]",
                result: None,
            },
        ]
    }
}

fn exists(path: &Path, span: Span, args: &Arguments) -> Value {
    let path = expand_path_with(path, &args.pwd);
    Value::Bool {
        val: match path.try_exists() {
            Ok(exists) => exists,
            Err(err) => {
                return Value::Error {
                    error: Box::new(ShellError::IOErrorSpanned(err.to_string(), span)),
                }
            }
        },
        span,
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
