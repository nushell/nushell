use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;
use std::fs::OpenOptions;
use std::path::PathBuf;

pub struct Touch;

#[derive(Deserialize)]
pub struct TouchArgs {
    pub target: Tagged<PathBuf>,
}

#[async_trait]
impl WholeStreamCommand for Touch {
    fn name(&self) -> &str {
        "touch"
    }
    fn signature(&self) -> Signature {
        Signature::build("touch").required(
            "filename",
            SyntaxShape::Path,
            "the path of the file you want to create",
        )
    }
    fn usage(&self) -> &str {
        "creates a file"
    }
    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        touch(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates \"fixture.json\"",
            example: "touch fixture.json",
            result: None,
        }]
    }
}

async fn touch(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let (TouchArgs { target }, _) = args.process(&registry).await?;

    match OpenOptions::new().write(true).create(true).open(&target) {
        Ok(_) => Ok(OutputStream::empty()),
        Err(err) => Err(ShellError::labeled_error(
            "File Error",
            err.to_string(),
            &target.tag,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::Touch;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Touch {})
    }
}
