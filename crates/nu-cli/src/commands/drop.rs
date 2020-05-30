use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

pub struct Drop;

#[derive(Deserialize)]
pub struct DropArgs {
    rows: Option<Tagged<u64>>,
}

#[async_trait]
impl WholeStreamCommand for Drop {
    fn name(&self) -> &str {
        "drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("drop").optional(
            "rows",
            SyntaxShape::Number,
            "starting from the back, the number of rows to drop",
        )
    }

    fn usage(&self) -> &str {
        "Drop the last number of rows."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let (DropArgs { rows }, input) = args.process(&registry).await?;
        let mut v: Vec<_> = input.into_vec().await;

        let rows_to_drop = if let Some(quantity) = rows {
            *quantity as usize
        } else {
            1
        };

        for _ in 0..rows_to_drop {
            v.pop();
        }

        Ok(futures::stream::iter(v).to_output_stream())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Remove the last item of a list/table",
                example: "echo [1 2 3] | drop",
                result: Some(vec![
                    UntaggedValue::int(1).into(),
                    UntaggedValue::int(2).into(),
                ]),
            },
            Example {
                description: "Remove the last 2 items of a list/table",
                example: "echo [1 2 3] | drop 2",
                result: Some(vec![UntaggedValue::int(1).into()]),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::Drop;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Drop {})
    }
}
