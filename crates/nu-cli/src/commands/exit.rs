use crate::commands::command::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CommandAction, ReturnSuccess, Signature};

pub struct Exit;

#[async_trait]
impl WholeStreamCommand for Exit {
    fn name(&self) -> &str {
        "exit"
    }

    fn signature(&self) -> Signature {
        Signature::build("exit").switch("now", "exit out of the shell immediately", Some('n'))
    }

    fn usage(&self) -> &str {
        "Exit the current shell (or all shells)"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        exit(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Exit the current shell",
                example: "exit",
                result: None,
            },
            Example {
                description: "Exit all shells (exiting Nu)",
                example: "exit --now",
                result: None,
            },
        ]
    }
}

pub fn exit(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let args = args.evaluate_once(&registry).await?;

        if args.call_info.args.has("now") {
            yield Ok(ReturnSuccess::Action(CommandAction::Exit));
        } else {
            yield Ok(ReturnSuccess::Action(CommandAction::LeaveShell));
        }
    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Exit;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Exit {})
    }
}
