use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::Signature;

pub struct Pwd;

#[async_trait]
impl WholeStreamCommand for Pwd {
    fn name(&self) -> &str {
        "pwd"
    }

    fn signature(&self) -> Signature {
        Signature::build("pwd")
    }

    fn usage(&self) -> &str {
        "Output the current working directory."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        pwd(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Print the current working directory",
            example: "pwd",
            result: None,
        }]
    }
}

pub async fn pwd(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let shell_manager = args.shell_manager.clone();
    let args = args.evaluate_once(&registry).await?;

    shell_manager.pwd(args)
}

#[cfg(test)]
mod tests {
    use super::Pwd;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Pwd {})
    }
}
