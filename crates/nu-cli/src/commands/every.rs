use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        every(args, registry).await
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

async fn every(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let (EveryArgs { stride, skip }, input) = args.process(&registry).await?;
    let v: Vec<_> = input.into_vec().await;

    let iter = v.into_iter().enumerate().filter_map(move |(i, x)| {
        let stride_desired = if stride.item < 1 { 1 } else { stride.item } as usize;
        let should_include = skip.item == (i % stride_desired != 0);

        if should_include {
            return Some(ReturnSuccess::value(x));
        }

        None
    });

    Ok(futures::stream::iter(iter).to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Every;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Every {})
    }
}
