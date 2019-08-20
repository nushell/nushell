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

fn to_array(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let span = args.call_info.name_span;
    let out = args.input.values.collect();

    Ok(out
        .map(move |vec: Vec<_>| stream![Value::List(vec).simple_spanned(span)])
        .flatten_stream()
        .from_input_stream())
}
