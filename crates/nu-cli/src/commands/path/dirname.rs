use super::{operate, DefaultArguments};
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue, Value};
use std::path::Path;

pub struct PathDirname;

#[derive(Deserialize)]
struct PathDirnameArguments {
    rest: Vec<ColumnPath>,
}

#[async_trait]
impl WholeStreamCommand for PathDirname {
    fn name(&self) -> &str {
        "path dirname"
    }

    fn signature(&self) -> Signature {
        Signature::build("path dirname").rest(SyntaxShape::ColumnPath, "optionally operate by path")
    }

    fn usage(&self) -> &str {
        "gets the dirname of a path"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (PathDirnameArguments { rest }, input) = args.process(&registry).await?;
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
            description: "Get dirname of a path",
            example: "echo '/home/joe/test.txt' | path dirname",
            result: Some(vec![Value::from("/home/joe")]),
        }]
    }
}

fn action(path: &Path, _args: Arc<DefaultArguments>) -> UntaggedValue {
    UntaggedValue::string(match path.parent() {
        Some(dirname) => dirname.to_string_lossy().to_string(),
        _ => "".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::PathDirname;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(PathDirname {})?)
    }
}
