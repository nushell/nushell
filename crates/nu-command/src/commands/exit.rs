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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        exit(args).await
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

pub async fn exit(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;

    let command_action = if args.call_info.args.has("now") {
        CommandAction::Exit
    } else {
        CommandAction::LeaveShell
    };

    Ok(OutputStream::one(ReturnSuccess::action(command_action)))
}

#[cfg(test)]
mod tests {
    use super::Exit;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Exit {})
    }
}
