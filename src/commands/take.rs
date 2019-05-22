use crate::errors::ShellError;
use crate::prelude::*;

// TODO: "Amount remaining" wrapper

pub fn take(args: CommandArgs<'caller>) -> Result<VecDeque<ReturnValue>, ShellError> {
    let amount = args.args[0].as_int()?;

    let amount = if args.input.len() > amount as usize {
        amount as usize
    } else {
        args.input.len()
    };

    let out: VecDeque<ReturnValue> = args
        .input
        .into_iter()
        .take(amount)
        .map(|v| ReturnValue::Value(v))
        .collect();

    Ok(out)
}
