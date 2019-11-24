use crate::commands::WholeStreamCommand;
use crate::prelude::*;

pub struct Debug;

#[derive(Deserialize)]
pub struct DebugArgs {}

impl WholeStreamCommand for Debug {
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
    RunnableContext { input, .. }: RunnableContext,
) -> Result<impl ToOutputStream, ShellError> {
    Ok(input
        .values
        .map(|v| ReturnSuccess::value(Value::string(format!("{:?}", v)).tagged_unknown()))
        .to_output_stream())
}
