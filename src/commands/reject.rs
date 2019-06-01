use crate::errors::ShellError;
use crate::object::base::reject_fields;
use crate::object::Value;
use crate::prelude::*;

pub fn reject(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.positional.is_empty() {
        return Err(ShellError::string("select requires a field"));
    }

    let fields: Result<Vec<String>, _> = args.positional.iter().map(|a| a.as_string()).collect();
    let fields = fields?;

    let stream = args
        .input
        .map(move |item| Value::Object(reject_fields(&item, &fields)))
        .map(|item| ReturnValue::Value(item));

    Ok(stream.boxed())
}
