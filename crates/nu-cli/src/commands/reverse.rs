use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature};

pub struct Reverse;

impl WholeStreamCommand for Reverse {
    fn name(&self) -> &str {
        "reverse"
    }

    fn signature(&self) -> Signature {
        Signature::build("reverse")
    }

    fn usage(&self) -> &str {
        "Reverses the table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        reverse(args, registry)
    }

    fn examples(&self) -> &[Example] {
        &[Example {
            description: "Sort files in descending file size",
            example: "ls | sort-by size | reverse",
        }]
    }
}

fn reverse(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let args = args.evaluate_once(&registry).await?;
        let (input, _args) = args.parts();

        let input = input.collect::<Vec<_>>().await;
        for output in input.into_iter().rev() {
            yield ReturnSuccess::value(output);
        }
    };

    Ok(stream.to_output_stream())
}
