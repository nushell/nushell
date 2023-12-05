use std::path::{Path, PathBuf};

use nu_engine::{current_dir, current_dir_const};
use nu_path::expand_path_with;
use nu_protocol::ast::Call;
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    engine::Command, Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

use super::PathSubcommandArguments;

struct Arguments {
    pwd: PathBuf,
}

impl PathSubcommandArguments for Arguments {}

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
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::Bool)),
                ),
            ])
            .category(Category::Path)
    }

    fn usage(&self) -> &str {
        "Check whether a path exists."
    }

    fn extra_usage(&self) -> &str {
        r#"This only checks if it is possible to either `open` or `cd` to the given path.
If you need to distinguish dirs and files, please use `path type`."#
    }

    fn is_const(&self) -> bool {
        true
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

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let args = Arguments {
            pwd: current_dir_const(working_set)?,
        };
        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&exists, &args, value, head),
            working_set.permanent().ctrlc.clone(),
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
                description: "Check if files in list exist",
                example: r"[ C:\joe\todo.txt, C:\Users\doe\todo.txt ] | path exists",
                result: Some(Value::test_list(vec![
                    Value::test_bool(false),
                    Value::test_bool(false),
                ])),
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
                description: "Check if files in list exist",
                example: "[ /home/joe/todo.txt, /home/doe/todo.txt ] | path exists",
                result: Some(Value::test_list(vec![
                    Value::test_bool(false),
                    Value::test_bool(false),
                ])),
            },
        ]
    }
}

fn exists(path: &Path, span: Span, args: &Arguments) -> Value {
    let path = expand_path_with(path, &args.pwd);
    Value::bool(
        match path.try_exists() {
            Ok(exists) => exists,
            Err(err) => {
                return Value::error(
                    ShellError::IOErrorSpanned {
                        msg: err.to_string(),
                        span,
                    },
                    span,
                )
            }
        },
        span,
    )
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
