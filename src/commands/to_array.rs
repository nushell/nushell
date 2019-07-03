use crate::object::Value;
use crate::prelude::*;

pub fn to_array(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let out = args.input.values.collect();

    Ok(out
        .map(|vec: Vec<_>| stream![Value::List(vec)])
        .flatten_stream()
        .from_input_stream())
}
