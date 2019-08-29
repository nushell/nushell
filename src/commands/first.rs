use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::parser::CommandRegistry;
use crate::prelude::*;

pub struct First;

impl WholeStreamCommand for First {
    fn name(&self) -> &str {
        "first"
    }

    fn signature(&self) -> Signature {
        Signature::build("first")
            .required("amount", SyntaxType::Literal)
    }

    fn usage(&self) -> &str {
        "Show only the first number of rows."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        first(args, registry)
    }
}

fn first(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;

    let amount = args.expect_nth(0)?.as_i64();

    let amount = match amount {
        Ok(o) => o,
        Err(_) => {
            return Err(ShellError::labeled_error(
                "Value is not a number",
                "expected integer",
                args.expect_nth(0)?.span(),
            ))
        }
    };

    Ok(OutputStream::from_input(
        args.input.values.take(amount as u64),
    ))
}
