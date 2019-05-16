use crate::errors::ShellError;
use crate::prelude::*;
use derive_new::new;

#[derive(new)]
pub struct Take;

// TODO: "Amount remaining" wrapper

impl crate::Command for Take {
    fn run(&self, args: CommandArgs<'caller>) -> Result<VecDeque<ReturnValue>, ShellError> {
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
}
