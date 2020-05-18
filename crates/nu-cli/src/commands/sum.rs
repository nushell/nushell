use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use crate::utils::data_processing::{reducer_for, Reduce};
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, ReturnValue, Signature, UntaggedValue, Value};
use num_traits::identities::Zero;

pub struct Sum;

impl WholeStreamCommand for Sum {
    fn name(&self) -> &str {
        "sum"
    }

    fn signature(&self) -> Signature {
        Signature::build("sum")
    }

    fn usage(&self) -> &str {
        "Sums the values."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        sum(RunnableContext {
            input: args.input,
            registry: registry.clone(),
            shell_manager: args.shell_manager,
            host: args.host,
            ctrl_c: args.ctrl_c,
            name: args.call_info.name_tag,
        })
    }

    fn examples(&self) -> &[Example] {
        &[
            Example {
                description: "Sum a list of numbers",
                example: "echo [1 2 3] | sum",
            },
            Example {
                description: "Get the disk usage for the current directory",
                example: "ls --all --du | get size | sum",
            },
        ]
    }
}

fn sum(RunnableContext { mut input, .. }: RunnableContext) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let mut values: Vec<Value> = input.drain_vec().await;
        let action = reducer_for(Reduce::Sum);

        if values.iter().all(|v| if let UntaggedValue::Primitive(_) = v.value {true} else {false}) {
            let total = action(Value::zero(), values)?;
            yield ReturnSuccess::value(total)
        } else {
            for value in values.into_iter() {
                let row_values = match value.value {
                    UntaggedValue::Row(row_dict) => {
                        Ok(row_dict.entries.into_iter().map(|kvp| kvp.1).collect())
                    },
                    _ => Err(ShellError::unimplemented("Can't compute the sum.")),
                };

                match row_values {
                    Ok(row_values) => {
                        let total = action(Value::zero(), row_values)?;
                        yield ReturnSuccess::value(total)
                    },
                    Err(err) => yield Err(err),
                }
            }
        }
    };

    let stream: BoxStream<'static, ReturnValue> = stream.boxed();

    Ok(stream.to_output_stream())
}
