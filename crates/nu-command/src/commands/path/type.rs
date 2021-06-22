use super::{operate, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::filesystem::filesystem_shell::get_file_type;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use std::path::Path;

pub struct PathType;

struct PathTypeArguments {
    rest: Vec<ColumnPath>,
}

impl PathSubcommandArguments for PathTypeArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath> {
        &self.rest
    }
}

impl WholeStreamCommand for PathType {
    fn name(&self) -> &str {
        "path type"
    }

    fn signature(&self) -> Signature {
        Signature::build("path type")
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
    }

    fn usage(&self) -> &str {
        "Get the type of the object a path refers to (e.g., file, dir, symlink)"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let cmd_args = Arc::new(PathTypeArguments {
            rest: args.rest(0)?,
        });

        Ok(operate(args.input, &action, tag.span, cmd_args))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Show type of a filepath",
            example: "echo '.' | path type",
            result: Some(vec![Value::from("Dir")]),
        }]
    }
}

fn action(path: &Path, tag: Tag, _args: &PathTypeArguments) -> Value {
    let meta = std::fs::symlink_metadata(path);
    let untagged = UntaggedValue::string(match &meta {
        Ok(md) => get_file_type(md),
        Err(_) => "",
    });

    untagged.into_value(tag)
}

#[cfg(test)]
mod tests {
    use super::PathType;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(PathType {})
    }
}
