use crate::errors::ShellError;
use crate::prelude::*;

pub fn sort_by(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let fields: Result<Vec<_>, _> = args.positional_iter().map(|a| a.as_string()).collect();
    let fields = fields?;

    let output = args.input.values.collect::<Vec<_>>();

    let output = output.map(move |mut vec| {
        vec.sort_by_key(|item| {
            fields
                .iter()
                .map(|f| item.get_data_by_key(f).map(|i| i.clone()))
                .collect::<Vec<Option<Spanned<Value>>>>()
        });

        vec.into_iter().collect::<VecDeque<_>>()
    });

    Ok(output.flatten_stream().from_input_stream())
}
