use super::{operate, DefaultArguments};
use crate::prelude::*;
use nu_engine::filesystem::filesystem_shell::get_file_type;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use std::path::Path;

pub struct PathType;

#[derive(Deserialize)]
struct PathTypeArguments {
    rest: Vec<ColumnPath>,
}

#[async_trait]
impl WholeStreamCommand for PathType {
    fn name(&self) -> &str {
        "path type"
    }

    fn signature(&self) -> Signature {
        Signature::build("path type")
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
    }

    fn usage(&self) -> &str {
        "Gives the type of the object a path refers to (e.g., file, dir, symlink)"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (PathTypeArguments { rest }, input) = args.process().await?;
        let args = Arc::new(DefaultArguments {
            replace: None,
            prefix: None,
            suffix: None,
            num_levels: None,
            paths: rest,
        });
        operate(input, &action, tag.span, args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Show type of a filepath",
            example: "echo '.' | path type",
            result: Some(vec![Value::from("Dir")]),
        }]
    }
}

fn action(path: &Path, _args: Arc<DefaultArguments>) -> UntaggedValue {
    let meta = std::fs::symlink_metadata(path);
    UntaggedValue::string(match &meta {
        Ok(md) => get_file_type(md),
        Err(_) => "",
    })
}

#[cfg(test)]
mod tests {
    use super::PathType;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(PathType {})
    }
}
