use crate::errors::ShellError;
use crate::prelude::*;

// TODO: "Amount remaining" wrapper

pub fn take(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let amount = args.args[0].as_i64()?;

    let input = args.input;

    Ok(input
        .take(amount as u64)
        .map(|v| ReturnValue::Value(v))
        .boxed())
}
