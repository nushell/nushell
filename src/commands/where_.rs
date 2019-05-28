use crate::errors::ShellError;
use crate::object::base::find;
use crate::prelude::*;

pub fn r#where(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.args.is_empty() {
        return Err(ShellError::string("select requires a field"));
    }

    let block = args.args[0].as_block()?;
    let input = args.input;

    let objects = input.filter_map(move |item| {
        let result = block.invoke(&item);

        let return_value = match result {
            Err(err) => {
                println!("{:?}", err);
                Some(ReturnValue::Value(Value::Error(Box::new(err))))
            }
            Ok(v) if v.is_true() => Some(ReturnValue::Value(item.copy())),
            _ => None,
        };

        futures::future::ready(return_value)
        // futures::future::ready(as_bool)
        // futures::future::ready(block.invoke(&item).
    });
    // .map(|item| )
    // .map(|item| ReturnValue::Value(item.copy()));

    Ok(objects.boxed())
}
