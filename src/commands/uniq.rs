use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use indexmap::set::IndexSet;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature};

#[derive(Deserialize)]
struct UniqArgs {}

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
        args.process(registry, uniq)?.run()
    }
}

fn uniq(
    UniqArgs {}: UniqArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let uniq_values: IndexSet<_> = input.values.collect().await;

        for item in uniq_values.iter().map(|row| ReturnSuccess::value(row.clone())) {
            yield item;
        }
    };

    Ok(stream.to_output_stream())
}
