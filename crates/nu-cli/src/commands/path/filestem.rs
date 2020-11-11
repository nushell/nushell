use super::{operate, DefaultArguments};
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use std::path::Path;

pub struct PathFilestem;

#[derive(Deserialize)]
struct PathFilestemArguments {
    rest: Vec<ColumnPath>,
}

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
        let (PathFilestemArguments { rest }, input) = args.process(&registry).await?;
        let args = Arc::new(DefaultArguments {
            replace: None,
            extension: None,
            num_levels: None,
            paths: rest,
        });
        operate(input, &action, tag.span, args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get filestem of a path",
            example: "echo '/home/joe/test.txt' | path filestem",
            result: Some(vec![Value::from("test")]),
        }]
    }
}

fn action(path: &Path, _args: Arc<DefaultArguments>) -> UntaggedValue {
    UntaggedValue::string(match path.file_stem() {
        Some(stem) => stem.to_string_lossy().to_string(),
        _ => "".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::PathFilestem;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(PathFilestem {})?)
    }
}
