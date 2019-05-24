use crate::errors::ShellError;
use crate::object::base::find;
use crate::prelude::*;

pub fn r#where(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.args.is_empty() {
        return Err(ShellError::string("select requires a field"));
    }

    let field: Result<String, _> = args.args[0].as_string();
    let field = field?;
    let input = args.input;
    let operator = args.args[1].copy();

    match operator {
        Value::Primitive(Primitive::Operator(operator)) => {
            let right = args.args[2].copy();

            let objects = input
                .filter(move |item| futures::future::ready(find(&item, &field, &operator, &right)))
                .map(|item| ReturnValue::Value(item.copy()));

            Ok(objects.boxed())
        }
        x => {
            println!("{:?}", x);
            Err(ShellError::string("expected a comparison operator"))
        }
    }
}
