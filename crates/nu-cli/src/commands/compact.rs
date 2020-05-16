use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use futures::stream::StreamExt;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
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
        compact(args, registry)
    }

    fn examples(&self) -> &[Example] {
        &[Example {
            description: "Remove all directory entries, except those with a 'target'",
            example: "ls -af | compact target",
        }]
    }
}

pub fn compact(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let (CompactArgs { rest: columns }, mut input) = args.process(&registry).await?;
        while let Some(item) = input.next().await {
            if columns.is_empty() {
                if !item.is_empty() {
                    yield ReturnSuccess::value(item);
                }
            } else {
                match item {
                    Value {
                        value: UntaggedValue::Row(ref r),
                        ..
                    } => if columns
                        .iter()
                        .all(|field| r.get_data(field).borrow().is_some()) {
                            yield ReturnSuccess::value(item);
                        }
                    _ => {},
                }
            };
        }
    };
    Ok(stream.to_output_stream())
}
