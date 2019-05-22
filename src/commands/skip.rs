use crate::errors::ShellError;
use crate::prelude::*;

pub fn skip(args: CommandArgs<'caller>) -> Result<VecDeque<ReturnValue>, ShellError> {
    let amount = args.args[0].as_int()?;

    let amount = if args.input.len() > amount as usize {
        amount as usize
    } else {
        args.input.len()
    };

    let out: VecDeque<ReturnValue> = args
        .input
        .into_iter()
        .skip(amount)
        .map(|v| ReturnValue::Value(v))
        .collect();

    Ok(out)
}
