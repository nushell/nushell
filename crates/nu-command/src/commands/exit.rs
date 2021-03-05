use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CommandAction, ReturnSuccess, Signature, SyntaxShape};

pub struct Exit;

#[async_trait]
impl WholeStreamCommand for Exit {
    fn name(&self) -> &str {
        "exit"
    }

    fn signature(&self) -> Signature {
        Signature::build("exit")
            .optional(
                "code",
                SyntaxShape::Number,
                "Status code to return if this was the last shell or --now was specified",
            )
            .switch("now", "Exit out of the shell immediately", Some('n'))
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

    let code = if let Some(value) = args.call_info.args.nth(0) {
        value.as_i32()?
    } else {
        0
    };

    let command_action = if args.call_info.args.has("now") {
        CommandAction::Exit(code)
    } else {
        CommandAction::LeaveShell(code)
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
