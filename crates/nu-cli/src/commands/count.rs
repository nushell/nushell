use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use futures::stream::StreamExt;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue, Value};

pub struct Count;

#[async_trait]
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

    async fn run(
        &self,
        args: CommandArgs,
        _registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let name = args.call_info.name_tag.clone();
        let rows: Vec<Value> = args.input.collect().await;

        Ok(OutputStream::one(
            UntaggedValue::int(rows.len()).into_value(name),
        ))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Count the number of entries in a list",
            example: "echo [1 2 3 4 5] | count",
            result: Some(vec![UntaggedValue::int(5).into()]),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::Count;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Count {})
    }
}
