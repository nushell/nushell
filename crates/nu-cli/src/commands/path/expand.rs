use super::{operate, DefaultArguments};
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue};
use std::path::Path;

pub struct PathExpand;

#[derive(Deserialize)]
struct PathExpandArguments {
    rest: Vec<ColumnPath>,
}

#[async_trait]
impl WholeStreamCommand for PathExpand {
    fn name(&self) -> &str {
        "path expand"
    }

    fn signature(&self) -> Signature {
        Signature::build("path expand").rest(SyntaxShape::ColumnPath, "optionally operate by path")
    }

    fn usage(&self) -> &str {
        "expands the path to its absolute form"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (PathExpandArguments { rest }, input) = args.process(&registry).await?;
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
            description: "Expand relative directories",
            example: "echo '/home/joe/foo/../bar' | path expand",
            result: None,
            //Some(vec![Value::from("/home/joe/bar")]),
        }]
    }
}

fn action(path: &Path, _args: Arc<DefaultArguments>) -> UntaggedValue {
    let ps = path.to_string_lossy();
    let expanded = shellexpand::tilde(&ps);
    let path: &Path = expanded.as_ref().as_ref();
    UntaggedValue::string(match path.canonicalize() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => ps.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::PathExpand;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(PathExpand {})?)
    }
}
