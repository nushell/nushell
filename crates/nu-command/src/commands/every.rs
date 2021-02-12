use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue};
use nu_source::Tagged;

pub struct Every;

#[derive(Deserialize)]
pub struct EveryArgs {
    stride: Tagged<u64>,
    skip: Tagged<bool>,
}

#[async_trait]
impl WholeStreamCommand for Every {
    fn name(&self) -> &str {
        "every"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "stride",
                SyntaxShape::Int,
                "how many rows to skip between (and including) each row returned",
            )
            .switch(
                "skip",
                "skip the rows that would be returned, instead of selecting them",
                Some('s'),
            )
    }

    fn usage(&self) -> &str {
        "Show (or skip) every n-th row, starting from the first one."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        every(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get every second row",
                example: "echo [1 2 3 4 5] | every 2",
                result: Some(vec![
                    UntaggedValue::int(1).into(),
                    UntaggedValue::int(3).into(),
                    UntaggedValue::int(5).into(),
                ]),
            },
            Example {
                description: "Skip every second row",
                example: "echo [1 2 3 4 5] | every 2 --skip",
                result: Some(vec![
                    UntaggedValue::int(2).into(),
                    UntaggedValue::int(4).into(),
                ]),
            },
        ]
    }
}

async fn every(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (EveryArgs { stride, skip }, input) = args.process().await?;

    let stride = stride.item;
    let skip = skip.item;

    Ok(input
        .enumerate()
        .filter_map(move |(i, value)| async move {
            let stride_desired = if stride < 1 { 1 } else { stride } as usize;
            let should_include = skip == (i % stride_desired != 0);

            if should_include {
                Some(ReturnSuccess::value(value))
            } else {
                None
            }
        })
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Every;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Every {})
    }
}
