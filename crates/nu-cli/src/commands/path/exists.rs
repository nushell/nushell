use super::{operate, DefaultArguments};
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};
use std::path::Path;

pub struct PathExists;

#[async_trait]
impl WholeStreamCommand for PathExists {
    fn name(&self) -> &str {
        "path exists"
    }

    fn signature(&self) -> Signature {
        Signature::build("path exists").rest(SyntaxShape::ColumnPath, "optionally operate by path")
    }

    fn usage(&self) -> &str {
        "checks whether the path exists"
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
            description: "Check if file exists",
            example: "echo '/home/joe/todo.txt' | path exists",
            result: Some(vec![Value::from(UntaggedValue::boolean(false))]),
        }]
    }
}

fn action(path: &Path) -> UntaggedValue {
    UntaggedValue::boolean(path.exists())
}
