use crate::errors::ShellError;
use crate::object::base::find;
use crate::prelude::*;

pub fn r#where(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.args.is_empty() {
        return Err(ShellError::string("select requires a field"));
    }

    let operation = args.args[0].as_operation()?;
    let field = operation.left.as_string()?;
    let operator = operation.operator;
    let right = operation.right;
    let input = args.input;

    let objects = input
        .filter(move |item| futures::future::ready(find(&item, &field, &operator, &right)))
        .map(|item| ReturnValue::Value(item.copy()));

    Ok(objects.boxed())
}
