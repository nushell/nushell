use super::PathSubcommandArguments;
use nu_engine::{command_prelude::*, current_dir, current_dir_const};
use nu_path::expand_path_with;
use nu_protocol::engine::StateWorkingSet;
use std::path::{Path, PathBuf};

struct Arguments {
    pwd: PathBuf,
    not_follow_symlink: bool,
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
            .switch("no-symlink", "Do not resolve symbolic links", Some('n'))
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
            not_follow_symlink: call.has_flag(engine_state, stack, "no-symlink")?,
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
            not_follow_symlink: call.has_flag_const(working_set, "no-symlink")?,
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
    if path.as_os_str().is_empty() {
        return Value::bool(false, span);
    }
    let path = expand_path_with(path, &args.pwd, true);
    let exists = if args.not_follow_symlink {
        // symlink_metadata returns true if the file/folder exists
        // whether it is a symbolic link or not. Sorry, but returns Err
        // in every other scenario including the NotFound
        std::fs::symlink_metadata(path).map_or_else(
            |e| match e.kind() {
                std::io::ErrorKind::NotFound => Ok(false),
                _ => Err(e),
            },
            |_| Ok(true),
        )
    } else {
        path.try_exists()
    };
    Value::bool(
        match exists {
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
