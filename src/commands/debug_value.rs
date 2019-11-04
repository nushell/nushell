use crate::commands::WholeStreamCommand;
use crate::prelude::*;

pub struct DebugValue;

#[derive(Deserialize)]
pub struct DebugArgs {}

impl WholeStreamCommand for DebugValue {
    fn name(&self) -> &str {
        "debug"
    }

    fn signature(&self) -> Signature {
        Signature::build("debug")
    }

    fn usage(&self) -> &str {
        "Print the Rust debug representation of the values"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, debug_value)?.run()
    }
}

fn debug_value(
    _args: DebugArgs,
    RunnableContext { mut input, .. }: RunnableContext,
) -> Result<impl ToOutputStream, ShellError> {
    let stream = async_stream! {
        while let Some(row) = input.values.next().await {
            yield ReturnSuccess::debug_value(row.clone())
        }
    };

    Ok(stream)
}
