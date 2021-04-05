use super::{operate, PathSubcommandArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use std::path::Path;

pub struct PathSplit;

#[derive(Deserialize)]
struct PathSplitArguments {
    rest: Vec<ColumnPath>,
}

impl PathSubcommandArguments for PathSplitArguments {
    fn get_column_paths(&self) -> &Vec<ColumnPath> {
        &self.rest
    }
}

#[async_trait]
impl WholeStreamCommand for PathSplit {
    fn name(&self) -> &str {
        "path split"
    }

    fn signature(&self) -> Signature {
        Signature::build("path split")
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
    }

    fn usage(&self) -> &str {
        "Split a path into parts along a separator."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (PathSplitArguments { rest }, input) = args.process().await?;
        let args = Arc::new(PathSplitArguments { rest });
        operate(input, &action, tag.span, args).await
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Split a path into parts",
            example: r"echo 'C:\Users\viking\spam.txt' | path split",
            result: Some(vec![Value::from(UntaggedValue::table(&[
                Value::from(UntaggedValue::string("C:")),
                Value::from(UntaggedValue::string("Users")),
                Value::from(UntaggedValue::string("viking")),
                Value::from(UntaggedValue::string("spam.txt")),
            ]))]),
        }]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Split a path into parts",
            example: r"echo '/home/viking/spam.txt' | path split",
            result: Some(vec![Value::from(UntaggedValue::table(&[
                Value::from(UntaggedValue::string("/")),
                Value::from(UntaggedValue::string("home")),
                Value::from(UntaggedValue::string("viking")),
                Value::from(UntaggedValue::string("spam.txt")),
            ]))]),
        }]
    }
}

fn action(path: &Path, tag: Tag, _args: &PathSplitArguments) -> Result<Value, ShellError> {
    let parts: Result<Vec<Value>, ShellError> = path
        .components()
        .map(|comp| match comp.as_os_str().to_str() {
            Some(s) => Ok(UntaggedValue::string(s).into_value(&tag)),
            None => Err(ShellError::untagged_runtime_error(
                "Error converting path component into UTF-8.",
            )),
        })
        .collect();

    Ok(UntaggedValue::table(&parts?).into_value(tag))
}

#[cfg(test)]
mod tests {
    use super::PathSplit;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(PathSplit {})
    }
}
