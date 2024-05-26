use super::PathSubcommandArguments;
use nu_engine::command_prelude::*;
use nu_protocol::engine::StateWorkingSet;
use std::path::{Path, PathBuf};

struct Arguments {
    pwd: PathBuf,
}

impl PathSubcommandArguments for Arguments {}

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "path type"
    }

    fn signature(&self) -> Signature {
        Signature::build("path type")
            .input_output_types(vec![
                (Type::String, Type::String),
                (
                    Type::List(Box::new(Type::String)),
                    Type::List(Box::new(Type::String)),
                ),
            ])
            .allow_variants_without_examples(true)
            .category(Category::Path)
    }

    fn usage(&self) -> &str {
        "Get the type of the object a path refers to (e.g., file, dir, symlink)."
    }

    fn extra_usage(&self) -> &str {
        r#"This checks the file system to confirm the path's object type.
If nothing is found, an empty string will be returned."#
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
            pwd: engine_state.cwd(Some(stack))?,
        };

        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&path_type, &args, value, head),
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
            pwd: working_set.permanent().cwd(None)?,
        };

        // This doesn't match explicit nulls
        if matches!(input, PipelineData::Empty) {
            return Err(ShellError::PipelineEmpty { dst_span: head });
        }
        input.map(
            move |value| super::operate(&path_type, &args, value, head),
            working_set.permanent().ctrlc.clone(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Show type of a filepath",
                example: "'.' | path type",
                result: Some(Value::test_string("dir")),
            },
            Example {
                description: "Show type of a filepaths in a list",
                example: "ls | get name | path type",
                result: None,
            },
        ]
    }
}

fn path_type(path: &Path, span: Span, args: &Arguments) -> Value {
    let path = nu_path::expand_path_with(path, &args.pwd, true);
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => Value::string(get_file_type(&metadata), span),
        Err(err) => Value::error(err.into(), span),
    }
}

fn get_file_type(md: &std::fs::Metadata) -> &str {
    let ft = md.file_type();
    let mut file_type = "unknown";
    if ft.is_dir() {
        file_type = "dir";
    } else if ft.is_file() {
        file_type = "file";
    } else if ft.is_symlink() {
        file_type = "symlink";
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileTypeExt;
            if ft.is_block_device() {
                file_type = "block device";
            } else if ft.is_char_device() {
                file_type = "char device";
            } else if ft.is_fifo() {
                file_type = "pipe";
            } else if ft.is_socket() {
                file_type = "socket";
            }
        }
    }
    file_type
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
