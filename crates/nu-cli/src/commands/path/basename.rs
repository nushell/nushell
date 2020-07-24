use super::operate;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue, Value};
use std::path::Path;

pub struct PathBasename;

#[async_trait]
impl WholeStreamCommand for PathBasename {
    fn name(&self) -> &str {
        "path filename"
    }

    fn signature(&self) -> Signature {
        Signature::build("path filename")
    }

    fn usage(&self) -> &str {
        "gets the filename of a path"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        operate(args, registry, action).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get filename of a path",
            example: "echo '/home/joe/test.txt' | path filename",
            result: Some(vec![Value::from("test.txt")]),
        }]
    }
}

fn action(path: &Path) -> UntaggedValue {
    UntaggedValue::string(match path.file_name() {
        Some(filename) => filename.to_string_lossy().to_string(),
        _ => "".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::PathBasename;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(PathBasename {})
    }
}
