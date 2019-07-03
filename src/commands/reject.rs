use crate::errors::ShellError;
use crate::object::base::reject_fields;
use crate::object::Value;
use crate::prelude::*;

pub fn reject(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.len() == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Reject requires fields",
            "needs parameter",
            args.name_span,
        ));
    }

    let fields: Result<Vec<String>, _> = args.positional_iter().map(|a| a.as_string()).collect();
    let fields = fields?;

    let stream = args
        .input
        .values
        .map(move |item| Value::Object(reject_fields(&item, &fields)));

    Ok(stream.from_input_stream())
}
