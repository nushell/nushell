use super::{operate, DefaultArguments};
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};
use std::path::Path;

pub struct PathExtension;

#[async_trait]
impl WholeStreamCommand for PathExtension {
    fn name(&self) -> &str {
        "path extension"
    }

    fn signature(&self) -> Signature {
        Signature::build("path extension")
            .rest(SyntaxShape::ColumnPath, "optionally operate by path")
    }

    fn usage(&self) -> &str {
        "gets the extension of a path"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (DefaultArguments { rest }, input) = args.process(&registry).await?;
        operate(input, rest, &action, tag.span).await
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
