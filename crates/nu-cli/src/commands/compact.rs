use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use futures::stream::StreamExt;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct Compact;

#[derive(Deserialize)]
pub struct CompactArgs {
    rest: Vec<Tagged<String>>,
}

impl WholeStreamCommand for Compact {
    fn name(&self) -> &str {
        "compact"
    }

    fn signature(&self) -> Signature {
        Signature::build("compact").rest(SyntaxShape::Any, "the columns to compact from the table")
    }

    fn usage(&self) -> &str {
        "Creates a table with non-empty rows"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, compact)?.run()
    }
}

pub fn compact(
    CompactArgs { rest: columns }: CompactArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let objects = input.values.filter(move |item| {
        let keep = if columns.is_empty() {
            item.is_some()
        } else {
            match item {
                Value {
                    value: UntaggedValue::Row(ref r),
                    ..
                } => columns
                    .iter()
                    .all(|field| r.get_data(field).borrow().is_some()),
                _ => false,
            }
        };

        futures::future::ready(keep)
    });

    Ok(objects.from_input_stream())
}
