use crate::errors::ShellError;
use crate::prelude::*;

pub fn sort_by(args: CommandArgs<'caller>) -> Result<VecDeque<ReturnValue>, ShellError> {
    let fields: Result<Vec<_>, _> = args.args.iter().map(|a| a.as_string()).collect();
    let fields = fields?;

    let mut output = args.input.into_iter().collect::<Vec<_>>();

    output.sort_by_key(|item| {
        fields
            .iter()
            .map(|f| item.get_data_by_key(f).borrow().copy())
            .collect::<Vec<Value>>()
    });

    let output = output
        .iter()
        .map(|o| ReturnValue::Value(o.copy()))
        .collect();

    Ok(output)
}
