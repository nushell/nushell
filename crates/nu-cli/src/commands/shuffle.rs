use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Value};

use rand::seq::SliceRandom;
use rand::thread_rng;

pub struct Shuffle;

#[async_trait]
impl WholeStreamCommand for Shuffle {
    fn name(&self) -> &str {
        "shuffle"
    }

    fn usage(&self) -> &str {
        "Shuffle rows randomly."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        shuffle(args, registry).await
    }
}

async fn shuffle(
    args: CommandArgs,
    _registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let input = args.input;
    let mut values: Vec<Value> = input.collect().await;

    values.shuffle(&mut thread_rng());

    Ok(futures::stream::iter(values.into_iter().map(ReturnSuccess::value)).to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Shuffle;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Shuffle {})
    }
}
