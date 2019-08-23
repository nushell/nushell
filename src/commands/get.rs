use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::object::Value;
use crate::prelude::*;

pub struct Get;

#[derive(Deserialize)]
pub struct GetArgs {
    rest: Vec<Tagged<String>>,
}

impl WholeStreamCommand for Get {
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
                // Before we give up, see if they gave us a path that matches a field name by itself
                match obj.get_data_by_key(&path.item) {
                    Some(v) => return Ok(v.clone()),
                    None => {
                        return Err(ShellError::labeled_error(
                            "Unknown column",
                            "table missing column",
                            path.span(),
                        ));
                    }
                }
            }
        }
    }

    Ok(current.clone())
}

pub fn get(
    GetArgs { rest: fields }: GetArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
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
