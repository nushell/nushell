use super::{operate, DefaultArguments};
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ColumnPath, Signature, SyntaxShape, UntaggedValue};
use std::path::{Path, PathBuf};

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
        Signature::build("path expand")
            .rest(SyntaxShape::ColumnPath, "Optionally operate by column path")
    }

    fn usage(&self) -> &str {
        "Expands a path to its absolute form"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (PathExpandArguments { rest }, input) = args.process().await?;
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
            description: "Expand relative directories",
            example: "echo 'C:\\Users\\joe\\foo\\..\\bar' | path expand",
            result: None,
            // fails to canonicalize into Some(vec![Value::from("C:\\Users\\joe\\bar")]) due to non-existing path
        }]
    }

    #[cfg(not(windows))]
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Expand relative directories",
            example: "echo '/home/joe/foo/../bar' | path expand",
            result: None,
            // fails to canonicalize into Some(vec![Value::from("/home/joe/bar")]) due to non-existing path
        }]
    }
}

fn action(path: &Path, _args: Arc<DefaultArguments>) -> UntaggedValue {
    let ps = path.to_string_lossy();
    let expanded = shellexpand::tilde(&ps);
    let path: &Path = expanded.as_ref().as_ref();
    UntaggedValue::filepath(dunce::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path)))
}

#[cfg(test)]
mod tests {
    use super::PathExpand;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(PathExpand {})
    }
}
