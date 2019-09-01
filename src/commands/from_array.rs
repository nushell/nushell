use crate::commands::WholeStreamCommand;
use crate::object::Value;
use crate::prelude::*;

pub struct FromArray;

impl WholeStreamCommand for FromArray {
    fn name(&self) -> &str {
        "from-array"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-array")
    }

    fn usage(&self) -> &str {
        "Expand an array/list into rows"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_array(args, registry)
    }
}

fn from_array(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let stream = args
        .input
        .values
        .map(|item| match item {
            Tagged {
                item: Value::List(vec),
                ..
            } => VecDeque::from(vec),
            x => VecDeque::from(vec![x]),
        })
        .flatten();

    Ok(stream.to_output_stream())
}
