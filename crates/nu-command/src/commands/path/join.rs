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
    path: Tagged<String>,
    rest: Vec<ColumnPath>,
}

impl PathSubcommandArguments for PathJoinArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath> {
        &self.rest
    }
}

#[async_trait]
impl WholeStreamCommand for PathJoin {
    fn name(&self) -> &str {
        "path join"
    }

    fn signature(&self) -> Signature {
        Signature::build("path join")
            .required("path", SyntaxShape::String, "Path to join the input path")
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
    }

    fn usage(&self) -> &str {
        "Joins an input path with another path"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (PathJoinArguments { path, rest }, input) = args.process().await?;
        let args = Arc::new(PathJoinArguments { path, rest });
        operate(input, &action, tag.span, args).await
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Append a filename to a path",
            example: "echo 'C:\\Users\\viking' | path join spam.txt",
            result: Some(vec![Value::from(UntaggedValue::filepath(
                "C:\\Users\\viking\\spam.txt",
            ))]),
        }]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Append a filename to a path",
            example: "echo '/home/viking' | path join spam.txt",
            result: Some(vec![Value::from(UntaggedValue::filepath(
                "/home/viking/spam.txt",
            ))]),
        }]
    }
}

fn action(path: &Path, args: &PathJoinArguments) -> UntaggedValue {
    UntaggedValue::filepath(path.join(&args.path.item))
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
