use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape};
use nu_source::Tagged;
use indexmap::set::IndexSet;

#[derive(Deserialize)]
struct UniqArgs {
    rest: Vec<Tagged<String>>,
}

pub struct Uniq;

impl WholeStreamCommand for Uniq {
    fn name(&self) -> &str {
        "uniq"
    }

    fn signature(&self) -> Signature {
        Signature::build("uniq")
            .rest(SyntaxShape::Any, "TODO: Figure out how to omit args")
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
    UniqArgs { rest: _fields }: UniqArgs,
    RunnableContext { input, .. } : RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let uniq_values: IndexSet<_> = input.values.collect().await;

        for item in uniq_values.iter().map(|row| ReturnSuccess::value(row.clone())) {
            yield item;
        }
    };

    Ok(stream.to_output_stream())
}

