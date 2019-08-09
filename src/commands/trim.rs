use crate::errors::ShellError;
use crate::object::Value;
use crate::prelude::*;

pub fn trim(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let input = args.input;

    Ok(input
        .values
        .map(move |v| {
            let string = String::extract(&v)?;
            ReturnSuccess::value(Value::string(string.trim()).simple_spanned(v.span()))
        })
        .to_output_stream())
}
