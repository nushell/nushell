use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use indexmap::set::IndexSet;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature};

pub struct Uniq;

impl WholeStreamCommand for Uniq {
    fn name(&self) -> &str {
        "uniq"
    }

    fn signature(&self) -> Signature {
        Signature::build("uniq")
    }

    fn usage(&self) -> &str {
        "Return the unique rows"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        uniq(args, registry)
    }
}

fn uniq(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let mut input = args.input;
        let uniq_values: IndexSet<_> = input.collect().await;

        for item in uniq_values.iter().map(|row| ReturnSuccess::value(row.clone())) {
            yield item;
        }
    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Uniq;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Uniq {})
    }
}
