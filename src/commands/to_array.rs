use crate::commands::WholeStreamCommand;
use crate::object::Value;
use crate::prelude::*;

pub struct ToArray;

impl WholeStreamCommand for ToArray {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_array(args, registry)
    }

    fn name(&self) -> &str {
        "to-array"
    }

    fn signature(&self) -> Signature {
        Signature::build("to-array")
    }
}

fn to_array(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let out = args.input.values.collect();

    Ok(out
        .map(|vec: Vec<_>| stream![Value::List(vec).tagged_unknown()]) // TODO: args.input should have a span
        .flatten_stream()
        .from_input_stream())
}
