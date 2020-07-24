use super::operate;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue, Value};
use std::path::Path;

pub struct PathExtension;

#[async_trait]
impl WholeStreamCommand for PathExtension {
    fn name(&self) -> &str {
        "path extension"
    }

    fn signature(&self) -> Signature {
        Signature::build("path extension")
    }

    fn usage(&self) -> &str {
        "gets the extension of a path"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        operate(args, registry, action).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get extension of a path",
                example: "echo 'test.txt' | path extension",
                result: Some(vec![Value::from("txt")]),
            },
            Example {
                description: "You get an empty string if there is no extension",
                example: "echo 'test' | path extension",
                result: Some(vec![Value::from("")]),
            },
        ]
    }
}

fn action(path: &Path) -> UntaggedValue {
    UntaggedValue::string(match path.extension() {
        Some(ext) => ext.to_string_lossy().to_string(),
        _ => "".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::PathExtension;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(PathExtension {})
    }
}
