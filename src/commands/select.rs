use crate::errors::ShellError;
use crate::object::Value;
use crate::object::base::select_fields;
use crate::prelude::*;

pub fn select(args: CommandArgs) -> Result<OutputStream, ShellError> {
    if args.args.is_empty() {
        return Err(ShellError::string("select requires a field"));
    }

    let fields: Result<Vec<String>, _> = args.args.iter().map(|a| a.as_string()).collect();
    let fields = fields?;

    let stream = args
        .input
        .map(move |item| {
            let mut result = VecDeque::new();
            let column = select_fields(&item, &fields);
            for field in &fields {
                match column.get_data_by_key(&field) {
                    Some(Value::List(l)) => {
                        for item in l {
                            result.push_back(ReturnValue::Value(item.copy()));
                        }
                    }
                    Some(x) => result.push_back(ReturnValue::Value(x.copy())),
                    None => {}
                }
            }

            result
        })
        .flatten();

    Ok(stream.boxed())
}
