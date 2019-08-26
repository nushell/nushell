use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::parser::CommandRegistry;
use crate::prelude::*;

pub struct Last;

impl WholeStreamCommand for Last {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        last(args, registry)
    }

    fn name(&self) -> &str {
        "last"
    }

    fn signature(&self) -> Signature {
        Signature::build("last").required("amount", SyntaxType::Literal)
    }
}

fn last(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
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

    if amount <= 0 {
        return Err(ShellError::labeled_error(
            "Value is too low",
            "expected a positive integer",
            args.expect_nth(0)?.span(),
        ));
    }

    let stream = async_stream_block! {
        let v: Vec<_> = args.input.into_vec().await;
        let k = v.len() - (amount as usize);
        for x in v[k..].iter() {
            let y: Tagged<Value> = x.clone();
            yield ReturnSuccess::value(y)
        }
    };
    Ok(stream.to_output_stream())
}
