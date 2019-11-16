use crate::data::Value;
use crate::errors::ShellError;
use crate::prelude::*;

use crate::parser::registry::Signature;

pub struct Echo;

impl PerItemCommand for Echo {
    fn name(&self) -> &str {
        "echo"
    }

    fn signature(&self) -> Signature {
        Signature::build("echo").rest(SyntaxShape::Any, "the values to echo")
    }

    fn usage(&self) -> &str {
        "Echo the arguments back to the user."
    }

    fn run(
        &self,
        call_info: &CallInfo,
        registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        _input: Tagged<Value>,
    ) -> Result<OutputStream, ShellError> {
        run(call_info, registry, raw_args)
    }
}

fn run(
    call_info: &CallInfo,
    _registry: &CommandRegistry,
    _raw_args: &RawCommandArgs,
) -> Result<OutputStream, ShellError> {
    let mut output = vec![];

    if let Some(ref positional) = call_info.args.positional {
        for i in positional {
            match i.as_string() {
                Ok(s) => {
                    output.push(Ok(ReturnSuccess::Value(
                        Value::string(s).tagged(i.tag.clone()),
                    )));
                }
                _ => match i {
                    Tagged {
                        item: Value::Table(table),
                        ..
                    } => {
                        for item in table {
                            output.push(Ok(ReturnSuccess::Value(item.clone())));
                        }
                    }
                    _ => {
                        output.push(Ok(ReturnSuccess::Value(i.clone())));
                    }
                },
            }
        }
    }

    let stream = VecDeque::from(output);

    Ok(stream.to_output_stream())
}
