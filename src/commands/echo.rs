use crate::commands::command::RunnablePerItemContext;
use crate::data::Value;
use crate::errors::ShellError;
use crate::parser::registry::Signature;
use crate::prelude::*;

pub struct Echo;
#[derive(Deserialize)]
pub struct EchoArgs {
    rest: Vec<Tagged<Value>>,
}

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
        _registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        _input: Tagged<Value>,
    ) -> Result<OutputStream, ShellError> {
        call_info.process(&raw_args.shell_manager, echo)?.run()
    }
}

fn echo(
    EchoArgs {
        rest: maybe_strings,
    }: EchoArgs,
    RunnablePerItemContext { name, .. }: &RunnablePerItemContext,
) -> Result<OutputStream, ShellError> {
    let mut output = String::new();

    for (idx, out) in maybe_strings.iter().enumerate() {
        match out.as_string() {
            Err(_) => {
                return Err(ShellError::type_error(
                    "a string-compatible value",
                    out.tagged_type_name(),
                ))
            }
            Ok(out) => {
                if idx > 0 {
                    output.push_str(" ");
                }

                output.push_str(&out);
            }
        }
    }

    let stream = VecDeque::from(vec![Ok(ReturnSuccess::Value(
        Value::string(output).tagged(name),
    ))]);

    Ok(stream.to_output_stream())
}
