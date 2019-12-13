use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::data::base::select_fields;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape};
use nu_source::Tagged;

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
            .rest(SyntaxShape::Any, "The columns to be unique over")
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
    UniqArgs { rest: fields }: UniqArgs,
    mut context: RunnableContext,
) -> Result<OutputStream, ShellError> {
    use std::collections::HashSet;

    Ok(OutputStream::new(async_stream! {
        let mut uniq_values = HashSet::new();

        let fields: Vec<_> = fields.iter().map(|field| field.item.clone()).collect();
        let vec = context.input.drain_vec().await;

        for item in vec {
            uniq_values.insert(item);
        }

        for item in uniq_values {
            yield item.into();
        }

    }))
}

