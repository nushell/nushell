use indexmap::IndexSet;
use nu_errors::ShellError;
use nu_protocol::{hir::CapturedBlock, ColumnPath, UntaggedValue, Value};
use nu_source::Tagged;
use nu_value_ext::ValueExt;

pub fn arguments(
    rest: &mut Vec<Value>,
) -> Result<(Vec<ColumnPath>, Option<Box<CapturedBlock>>), ShellError> {
    let last_argument = rest.pop();

    let mut columns: IndexSet<_> = rest.iter().collect();
    let mut column_paths = vec![];

    let mut default = None;

    for argument in columns.drain(..) {
        let Tagged { item: path, .. } = argument.as_column_path()?;

        column_paths.push(path);
    }

    match last_argument {
        Some(Value {
            value: UntaggedValue::Block(call),
            ..
        }) => default = Some(call),
        Some(other) => {
            let Tagged { item: path, .. } = other.as_column_path()?;

            column_paths.push(path);
        }
        None => {}
    };

    Ok((column_paths, default))
}
