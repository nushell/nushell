use crate::errors::ShellError;
use crate::object::base::reject_fields;
use crate::prelude::*;

pub fn reject(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.len() == 0 {
        return Err(ShellError::labeled_error(
            "Reject requires fields",
            "needs parameter",
            args.call_info.name_span,
        ));
    }

    let fields: Result<Vec<String>, _> = args.positional_iter().map(|a| a.as_string()).collect();
    let fields = fields?;

    let stream = args
        .input
        .values
        .map(move |item| reject_fields(&item, &fields, item.tag()).into_tagged_value());

    Ok(stream.from_input_stream())
}
