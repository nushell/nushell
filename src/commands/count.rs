use crate::commands::WholeStreamCommand;
use crate::data::Value;
use crate::errors::ShellError;
use crate::parser::CommandRegistry;
use crate::prelude::*;
use futures::stream::StreamExt;

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
        "Show the total number of cells."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, count)?.run()
    }
}

pub fn count(
    CountArgs {}: CountArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let rows: Vec<Tagged<Value>> = input.values.collect().await;

        yield ReturnSuccess::value(Value::int(rows.len()).tagged(name))
    };

    Ok(stream.to_output_stream())
}
