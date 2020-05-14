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
        echo(args, registry)
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

fn echo(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let (args, _): (EchoArgs, _) = args.process(&registry).await?;

        for i in args.rest {
            match i.as_string() {
                Ok(s) => {
                    yield Ok(ReturnSuccess::Value(
                        UntaggedValue::string(s).into_value(i.tag.clone()),
                    ));
                }
                _ => match i {
                    Value {
                        value: UntaggedValue::Table(table),
                        ..
                    } => {
                        for value in table {
                            yield Ok(ReturnSuccess::Value(value.clone()));
                        }
                    }
                    _ => {
                        yield Ok(ReturnSuccess::Value(i.clone()));
                    }
                },
            }
        }
    };

    Ok(stream.to_output_stream())
}
