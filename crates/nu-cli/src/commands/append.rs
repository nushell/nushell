use crate::command_registry::CommandRegistry;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};

#[derive(Deserialize)]
struct Arguments {
    row: Value,
}

pub struct Command;

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "append"
    }

    fn signature(&self) -> Signature {
        Signature::build("append").required(
            "row value",
            SyntaxShape::Any,
            "the value of the row to append to the table",
        )
    }

    fn usage(&self) -> &str {
        "Append the given row to the table"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let (Arguments { mut row }, input) = args.process(registry).await?;

        let input: Vec<Value> = input.collect().await;

        if let Some(first) = input.get(0) {
            row.tag = first.tag();
        }

        Ok(
            futures::stream::iter(input.into_iter().chain(vec![row]).map(ReturnSuccess::value))
                .to_output_stream(),
        )
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Add something to the end of a list or table",
            example: "echo [1 2 3] | append 4",
            result: Some(vec![
                UntaggedValue::int(1).into(),
                UntaggedValue::int(2).into(),
                UntaggedValue::int(3).into(),
                UntaggedValue::int(4).into(),
            ]),
        }]
    }
}
