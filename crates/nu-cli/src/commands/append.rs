use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};

#[derive(Deserialize)]
struct AppendArgs {
    row: Value,
}

pub struct Append;

#[async_trait]
impl WholeStreamCommand for Append {
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
        let (AppendArgs { row }, input) = args.process(registry).await?;

        let eos = futures::stream::iter(vec![row]);

        Ok(input.chain(eos).to_output_stream())
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

#[cfg(test)]
mod tests {
    use super::Append;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Append {})
    }
}
