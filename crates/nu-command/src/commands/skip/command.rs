use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

pub struct Command;

#[derive(Deserialize)]
pub struct Arguments {
    rows: Option<Tagged<usize>>,
}

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "skip"
    }

    fn signature(&self) -> Signature {
        Signature::build("skip").optional("rows", SyntaxShape::Int, "How many rows to skip")
    }

    fn usage(&self) -> &str {
        "Skip some number of rows."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        skip(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Skip the first 5 rows",
            example: "echo [1 2 3 4 5 6 7] | skip 5",
            result: Some(vec![
                UntaggedValue::int(6).into(),
                UntaggedValue::int(7).into(),
            ]),
        }]
    }
}

async fn skip(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (Arguments { rows }, input) = args.process().await?;
    let rows_desired = if let Some(quantity) = rows {
        *quantity
    } else {
        1
    };

    Ok(input.skip(rows_desired).to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Command;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Command {})
    }
}
