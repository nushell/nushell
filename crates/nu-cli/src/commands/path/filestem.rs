use super::{operate, DefaultArguments};
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};
use std::path::Path;

pub struct PathFilestem;

#[async_trait]
impl WholeStreamCommand for PathFilestem {
    fn name(&self) -> &str {
        "path filestem"
    }

    fn signature(&self) -> Signature {
        Signature::build("path filestem")
            .rest(SyntaxShape::ColumnPath, "optionally operate by path")
    }

    fn usage(&self) -> &str {
        "gets the filestem of a path"
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
        vec![Example {
            description: "Get filestem of a path",
            example: "echo '/home/joe/test.txt' | path filestem",
            result: Some(vec![Value::from("test")]),
        }]
    }
}

fn action(path: &Path) -> UntaggedValue {
    UntaggedValue::string(match path.file_stem() {
        Some(stem) => stem.to_string_lossy().to_string(),
        _ => "".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::PathFilestem;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(PathFilestem {})
    }
}
