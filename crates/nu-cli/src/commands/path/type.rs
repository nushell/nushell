use super::{operate, DefaultArguments};
use crate::commands::WholeStreamCommand;
use crate::data::files::get_file_type;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};
use std::path::Path;

pub struct PathType;

#[async_trait]
impl WholeStreamCommand for PathType {
    fn name(&self) -> &str {
        "path type"
    }

    fn signature(&self) -> Signature {
        Signature::build("path type").rest(SyntaxShape::ColumnPath, "optionally operate by path")
    }

    fn usage(&self) -> &str {
        "gives the type of the object the path refers to (eg file, dir, symlink)"
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
            description: "Show type of a filepath",
            example: "echo '.' | path type",
            result: Some(vec![Value::from("Dir")]),
        }]
    }
}

fn action(path: &Path) -> UntaggedValue {
    let meta = std::fs::symlink_metadata(path);
    UntaggedValue::string(match &meta {
        Ok(md) => get_file_type(md),
        Err(_) => "",
    })
}

#[cfg(test)]
mod tests {
    use super::PathType;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(PathType {})
    }
}
