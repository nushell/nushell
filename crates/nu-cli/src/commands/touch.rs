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
    target: Tagged<PathBuf>,
    rest: Vec<Tagged<PathBuf>>,
}

#[async_trait]
impl WholeStreamCommand for Touch {
    fn name(&self) -> &str {
        "touch"
    }
    fn signature(&self) -> Signature {
        Signature::build("touch")
            .required(
                "filename",
                SyntaxShape::Path,
                "the path of the file you want to create",
            )
            .rest(SyntaxShape::Path, "additional files to create")
    }
    fn usage(&self) -> &str {
        "creates one or more files"
    }
    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        touch(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Creates \"fixture.json\"",
                example: "touch fixture.json",
                result: None,
            },
            Example {
                description: "Creates files a, b and c",
                example: "touch a b c",
                result: None,
            },
        ]
    }
}

async fn touch(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let (TouchArgs { target, rest }, _) = args.process(&registry).await?;

    for item in vec![target].into_iter().chain(rest.into_iter()) {
        match OpenOptions::new().write(true).create(true).open(&item) {
            Ok(_) => continue,
            Err(err) => {
                return Err(ShellError::labeled_error(
                    "File Error",
                    err.to_string(),
                    &item.tag,
                ))
            }
        }
    }

    Ok(OutputStream::empty())
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
