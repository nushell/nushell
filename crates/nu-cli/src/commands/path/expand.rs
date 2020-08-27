use super::{operate, DefaultArguments};
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};
use std::path::Path;

pub struct PathExpand;

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
        let (DefaultArguments { rest }, input) = args.process(&registry).await?;
        operate(input, rest, &action, tag.span).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Expand relative directories",
            example: "echo '/home/joe/foo/../bar' | path expand",
            result: Some(vec![Value::from("/home/joe/bar")]),
        }]
    }
}

fn action(path: &Path) -> UntaggedValue {
    let ps = path.to_string_lossy();
    let expanded = shellexpand::tilde(&ps);
    let path: &Path = expanded.as_ref().as_ref();
    UntaggedValue::string(match path.canonicalize() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => ps.to_string(),
    })
}
