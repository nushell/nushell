use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};

pub struct Echo;

#[derive(Deserialize)]
pub struct EchoArgs {
    pub rest: Vec<Value>,
}

impl WholeStreamCommand for Echo {
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
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, echo)?.run()
    }

    fn examples(&self) -> &[Example] {
        &[
            Example {
                description: "Put a hello message in the pipeline",
                example: "echo 'hello'",
            },
            Example {
                description: "Print the value of the special '$nu' variable",
                example: "echo $nu",
            },
        ]
    }
}

fn echo(args: EchoArgs, _: RunnableContext) -> Result<OutputStream, ShellError> {
    let mut output = vec![];

    for i in args.rest {
        match i.as_string() {
            Ok(s) => {
                output.push(Ok(ReturnSuccess::Value(
                    UntaggedValue::string(s).into_value(i.tag.clone()),
                )));
            }
            _ => match i {
                Value {
                    value: UntaggedValue::Table(table),
                    ..
                } => {
                    for value in table {
                        output.push(Ok(ReturnSuccess::Value(value.clone())));
                    }
                }
                _ => {
                    output.push(Ok(ReturnSuccess::Value(i.clone())));
                }
            },
        }
    }

    // TODO: This whole block can probably be replaced with `.map()`
    let stream = futures::stream::iter(output);

    Ok(stream.to_output_stream())
}
