use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use futures::stream::StreamExt;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue, Value};

pub struct Count;

#[derive(Deserialize)]
pub struct CountArgs {}

impl WholeStreamCommand for Count {
    fn name(&self) -> &str {
        "count"
    }

    fn signature(&self) -> Signature {
        Signature::build("count")
    }

    fn usage(&self) -> &str {
        "Show the total number of rows or items."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, count)?.run()
    }

    fn examples(&self) -> &[Example] {
        &[Example {
            description: "Count the number of files/directories in the current directory",
            example: "ls | count",
        }]
    }
}

pub fn count(
    CountArgs {}: CountArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let rows: Vec<Value> = input.collect().await;

        yield ReturnSuccess::value(UntaggedValue::int(rows.len()).into_value(name))
    };

    Ok(stream.to_output_stream())
}
