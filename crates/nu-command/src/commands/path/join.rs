use super::{operate, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::path::Path;

pub struct PathJoin;

#[derive(Deserialize)]
struct PathJoinArguments {
    rest: Vec<ColumnPath>,
    appendix: Option<Tagged<String>>,
}

impl PathSubcommandArguments for PathJoinArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath> {
        &self.rest
    }
}

impl WholeStreamCommand for PathJoin {
    fn name(&self) -> &str {
        "path join"
    }

    fn signature(&self) -> Signature {
        Signature::build("path join")
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
            .named(
                "appendix",
                SyntaxShape::String,
                "Path to append to the input",
                Some('a'),
            )
    }

    fn usage(&self) -> &str {
        "Join a structured path or a list of path parts."
    }

    fn extra_usage(&self) -> &str {
        "Optionally, append additional to the result. It is designed to accept the output of 'path
parse' and 'path split' subdommands."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (PathJoinArguments { rest, appendix }, input) = args.process()?;
        let args = Arc::new(PathJoinArguments { rest, appendix });
        Ok(operate(input, &action, tag.span, args))
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Append a filename to a path",
            example: "echo 'C:\\Users\\viking' | path join -a spam.txt",
            result: Some(vec![Value::from(UntaggedValue::filepath(
                "C:\\Users\\viking\\spam.txt",
            ))]),
        }]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Append a filename to a path",
            example: "echo '/home/viking' | path join -a spam.txt",
            result: Some(vec![Value::from(UntaggedValue::filepath(
                "/home/viking/spam.txt",
            ))]),
        }]
    }
}

#[allow(clippy::unnecessary_wraps)]
fn action(path: &Path, tag: Tag, args: &PathJoinArguments) -> Result<Value, ShellError> {
    if let Some(ref appendix) = args.appendix {
        Ok(UntaggedValue::filepath(path.join(&appendix.item)).into_value(tag))
    } else {
        Ok(UntaggedValue::filepath(path).into_value(tag))
    }
}

#[cfg(test)]
mod tests {
    use super::PathJoin;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(PathJoin {})
    }
}
