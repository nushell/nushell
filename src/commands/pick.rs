use crate::errors::ShellError;
use crate::object::base::select_fields;
use crate::prelude::*;

pub fn pick(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.len() == 0 {
        return Err(ShellError::labeled_error(
            "Pick requires fields",
            "needs parameter",
            args.call_info.name_span,
        ));
    }

    let fields: Result<Vec<String>, _> = args.positional_iter().map(|a| a.as_string()).collect();
    let fields = fields?;
    let input = args.input;

    let objects = input
        .values
        .map(move |value| select_fields(&value.item, &fields, value.tag()));

    Ok(objects.from_input_stream())
}
