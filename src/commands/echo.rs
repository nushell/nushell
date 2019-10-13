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
        Signature::build("echo").rest(SyntaxShape::Any)
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
    let name = call_info.name_tag.clone();

    let mut output = String::new();

    let mut first = true;

    if let Some(ref positional) = call_info.args.positional {
        for i in positional {
            match i.as_string() {
                Ok(s) => {
                    if !first {
                        output.push_str(" ");
                    } else {
                        first = false;
                    }

                    output.push_str(&s);
                }
                _ => {
                    return Err(ShellError::type_error(
                        "a string-compatible value",
                        i.tagged_type_name(),
                    ))
                }
            }
        }
    }

    let stream = VecDeque::from(vec![Ok(ReturnSuccess::Value(
        Value::string(output).tagged(name),
    ))]);

    Ok(stream.to_output_stream())
}
