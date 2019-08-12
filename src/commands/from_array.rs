use crate::object::Value;
use crate::prelude::*;

pub fn from_array(
    args: CommandArgs,
    _registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
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
