use crate::commands::StaticCommand;
use crate::errors::ShellError;
use crate::object::Value;
use crate::prelude::*;

pub struct Get;

#[derive(Deserialize)]
pub struct GetArgs {
    rest: Vec<Tagged<String>>,
}

impl StaticCommand for Get {
    fn name(&self) -> &str {
        "get"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, get)?.run()
    }
    fn signature(&self) -> Signature {
        Signature::build("get").rest()
    }
}

fn get_member(path: &Tagged<String>, obj: &Tagged<Value>) -> Result<Tagged<Value>, ShellError> {
    let mut current = obj;
    for p in path.split(".") {
        match current.get_data_by_key(p) {
            Some(v) => current = v,
            None => {
                return Err(ShellError::labeled_error(
                    "Unknown field",
                    "object missing field",
                    path.span(),
                ));
            }
        }
    }

    Ok(current.clone())
}

pub fn get(
    GetArgs { rest: fields }: GetArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    // If it's a number, get the row instead of the column
    // if let Some(amount) = amount {
    //     return Ok(input.values.skip(amount as u64).take(1).from_input_stream());
    // }

    let stream = input
        .values
        .map(move |item| {
            let mut result = VecDeque::new();
            for field in &fields {
                match get_member(field, &item) {
                    Ok(Tagged {
                        item: Value::List(l),
                        ..
                    }) => {
                        for item in l {
                            result.push_back(ReturnSuccess::value(item.clone()));
                        }
                    }
                    Ok(x) => result.push_back(ReturnSuccess::value(x.clone())),
                    Err(x) => result.push_back(Err(x)),
                }
            }

            result
        })
        .flatten();

    Ok(stream.to_output_stream())
}
