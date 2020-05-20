use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, ReturnValue, Value};

use rand::seq::SliceRandom;
use rand::thread_rng;

pub struct Shuffle;

impl WholeStreamCommand for Shuffle {
    fn name(&self) -> &str {
        "shuffle"
    }

    fn usage(&self) -> &str {
        "Shuffle rows randomly."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        shuffle(args, registry)
    }
}

fn shuffle(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let mut input = args.input;
        let mut values: Vec<Value> = input.collect().await;

        let out = {
            values.shuffle(&mut thread_rng());
            values.clone()
        };

        for val in out.into_iter() {
            yield ReturnSuccess::value(val);
        }
    };

    let stream: BoxStream<'static, ReturnValue> = stream.boxed();

    Ok(stream.to_output_stream())
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
