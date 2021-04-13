use super::{operate, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use std::path::{Path, PathBuf};

pub struct PathExpand;

struct PathExpandArguments {
    rest: Vec<ColumnPath>,
}

impl PathSubcommandArguments for PathExpandArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath> {
        &self.rest
    }
}

impl WholeStreamCommand for PathExpand {
    fn name(&self) -> &str {
        "path expand"
    }

    fn signature(&self) -> Signature {
        Signature::build("path expand")
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
    }

    fn usage(&self) -> &str {
        "Expand a path to its absolute form"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let args = args.evaluate_once()?;
        let cmd_args = Arc::new(PathExpandArguments {
            rest: args.rest_args()?,
        });

        Ok(operate(args.input, &action, tag.span, cmd_args))
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Expand relative directories",
            example: "echo 'C:\\Users\\joe\\foo\\..\\bar' | path expand",
            result: None,
            // fails to canonicalize into Some(vec![Value::from("C:\\Users\\joe\\bar")]) due to non-existing path
        }]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Expand relative directories",
            example: "echo '/home/joe/foo/../bar' | path expand",
            result: None,
            // fails to canonicalize into Some(vec![Value::from("/home/joe/bar")]) due to non-existing path
        }]
    }
}

fn action(path: &Path, tag: Tag, _args: &PathExpandArguments) -> Value {
    let ps = path.to_string_lossy();
    let expanded = shellexpand::tilde(&ps);
    let path: &Path = expanded.as_ref().as_ref();

    UntaggedValue::filepath(dunce::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path)))
        .into_value(tag)
}

#[cfg(test)]
mod tests {
    use super::PathExpand;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(PathExpand {})
    }
}
