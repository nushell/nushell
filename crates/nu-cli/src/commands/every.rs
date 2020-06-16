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
                "every which row to select",
            )
    }

    fn usage(&self) -> &str {
        "Show only every nth row, starting from the first."
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
        ]
    }
}

async fn every(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let (EveryArgs { stride }, input) = args.process(&registry).await?;
    let v: Vec<_> = input.into_vec().await;

    let stride_desired = if stride.item < 1 { 1 } else { stride.item } as usize;

    let mut values_vec_deque = VecDeque::new();

    for (i, x) in v.iter().enumerate() {
        if i % stride_desired == 0 {
            values_vec_deque.push_back(ReturnSuccess::value(x.clone()));
        }
    }

    Ok(futures::stream::iter(values_vec_deque).to_output_stream())
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
