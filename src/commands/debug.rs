use crate::commands::WholeStreamCommand;
use crate::prelude::*;

pub struct Debug;

#[derive(Deserialize)]
pub struct DebugArgs {
    raw: Tagged<bool>,
}

impl WholeStreamCommand for Debug {
    fn name(&self) -> &str {
        "debug"
    }

    fn signature(&self) -> Signature {
        Signature::build("debug").switch("raw", "print raw data")
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
    DebugArgs { raw }: DebugArgs,
    RunnableContext { mut input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        while let Some(row) = input.values.next().await {
            if let Tagged { item: true, .. } = raw {      
                println!("{:?}", row);
                yield ReturnSuccess::value(row)
            } else {
                yield ReturnSuccess::debug_value(row.clone())
            }
        }
    };

    Ok(stream.to_output_stream())
}
