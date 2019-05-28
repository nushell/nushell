use crate::errors::ShellError;
use crate::prelude::*;

pub fn sort_by(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let fields: Result<Vec<_>, _> = args.args.iter().map(|a| a.as_string()).collect();
    let fields = fields?;

    let output = args.input.collect::<Vec<_>>();

    let output = output.map(move |mut vec| {
        vec.sort_by_key(|item| {
            fields
                .iter()
                .map(|f| item.get_data_by_key(f).map(|i| i.copy()))
                .collect::<Vec<Option<Value>>>()
        });

        vec.into_iter()
            .map(|v| ReturnValue::Value(v.copy()))
            .collect::<VecDeque<_>>()
            .boxed()
    });

    Ok(output.flatten_stream().boxed())
}
