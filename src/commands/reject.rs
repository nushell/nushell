use crate::errors::ShellError;
use crate::object::base::reject_fields;
use crate::prelude::*;

pub fn reject(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name_span = args.call_info.name_span;

    if args.len() == 0 {
        return Err(ShellError::maybe_labeled_error(
            "Reject requires fields",
            "needs parameter",
            args.call_info.name_span,
        ));
    }

    let fields: Result<Vec<String>, _> = args.positional_iter().map(|a| a.as_string()).collect();
    let fields = fields?;

    let stream = args.input.values.map(move |item| {
        reject_fields(&item, &fields, item.span)
            .into_spanned_value()
            .spanned(name_span)
    });

    Ok(stream.from_input_stream())
}
