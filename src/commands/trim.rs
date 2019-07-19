use crate::errors::ShellError;
use crate::object::Value;
use crate::prelude::*;

pub fn trim(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let input = args.input;

    Ok(input
        .values
        .map(move |v| {
            let string = String::extract(&v)?;
            ReturnSuccess::value(Value::string(string.trim()).spanned(v.span))
        })
        .to_output_stream())
}
