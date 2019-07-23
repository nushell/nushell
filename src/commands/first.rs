use crate::errors::ShellError;
use crate::parser::CommandRegistry;
use crate::prelude::*;

// TODO: "Amount remaining" wrapper

pub fn first(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;

    if args.len() == 0 {
        return Err(ShellError::maybe_labeled_error(
            "First requires an amount",
            "needs parameter",
            args.name_span(),
        ));
    }

    let amount = args.expect_nth(0)?.as_i64();

    let amount = match amount {
        Ok(o) => o,
        Err(_) => {
            return Err(ShellError::labeled_error(
                "Value is not a number",
                "expected integer",
                args.expect_nth(0)?.span,
            ))
        }
    };

    Ok(OutputStream::from_input(
        args.input.values.take(amount as u64),
    ))
}
