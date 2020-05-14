use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use futures::stream::StreamExt;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue, Value};

pub struct Count;

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
        count(args, registry)
    }

    fn examples(&self) -> &[Example] {
        &[Example {
            description: "Count the number of files/directories in the current directory",
            example: "ls | count",
        }]
    }
}

pub fn count(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let name = args.call_info.name_tag.clone();
        let rows: Vec<Value> = args.input.collect().await;

        yield ReturnSuccess::value(UntaggedValue::int(rows.len()).into_value(name))
    };

    Ok(stream.to_output_stream())
}
