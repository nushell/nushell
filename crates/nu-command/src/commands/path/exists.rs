use super::{operate, DefaultArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use std::path::Path;

pub struct PathExists;

#[derive(Deserialize)]
struct PathExistsArguments {
    rest: Vec<ColumnPath>,
}

#[async_trait]
impl WholeStreamCommand for PathExists {
    fn name(&self) -> &str {
        "path exists"
    }

    fn signature(&self) -> Signature {
        Signature::build("path exists")
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
    }

    fn usage(&self) -> &str {
        "Checks whether a path exists"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (PathExistsArguments { rest }, input) = args.process().await?;
        let args = Arc::new(DefaultArguments {
            replace: None,
            prefix: None,
            suffix: None,
            num_levels: None,
            paths: rest,
        });
        operate(input, &action, tag.span, args).await
    }

    #[cfg(windows)]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Check if file exists",
            example: "echo 'C:\\Users\\joe\\todo.txt' | path exists",
            result: Some(vec![Value::from(UntaggedValue::boolean(false))]),
        }]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Check if file exists",
            example: "echo '/home/joe/todo.txt' | path exists",
            result: Some(vec![Value::from(UntaggedValue::boolean(false))]),
        }]
    }
}

fn action(path: &Path, _args: Arc<DefaultArguments>) -> UntaggedValue {
    UntaggedValue::boolean(path.exists())
}

#[cfg(test)]
mod tests {
    use super::PathExists;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(PathExists {})
    }
}
