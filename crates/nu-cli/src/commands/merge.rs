use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::data::value::merge_values;
use crate::prelude::*;

use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
pub struct Merge;

#[derive(Deserialize)]
pub struct MergeArgs {
    block: Block,
}

impl WholeStreamCommand for Merge {
    fn name(&self) -> &str {
        "merge"
    }

    fn signature(&self) -> Signature {
        Signature::build("merge").required(
            "block",
            SyntaxShape::Block,
            "the block to run and merge into the table",
        )
    }

    fn usage(&self) -> &str {
        "Merge a table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        Ok(args.process_raw(registry, merge)?.run())
    }

    fn examples(&self) -> &[Example] {
        &[Example {
            description: "Merge a 1-based index column with some ls output",
            example: "ls | select name | keep 3 | merge { echo [1 2 3] | wrap index }",
        }]
    }
}

fn merge(
    merge_args: MergeArgs,
    context: RunnableContext,
    raw_args: RawCommandArgs,
) -> Result<OutputStream, ShellError> {
    let block = merge_args.block;
    let registry = context.registry.clone();
    let mut input = context.input;
    let scope = raw_args.call_info.scope.clone();

    let mut context = Context::from_raw(&raw_args, &registry);

    let stream = async_stream! {
        let table: Option<Vec<Value>> = match run_block(&block,
                &mut context,
                InputStream::empty(),
                &scope).await {
            Ok(mut stream) => Some(stream.drain_vec().await),
            Err(err) => {
                yield Err(err);
                return;
            }
        };


        let table = table.unwrap_or_else(|| vec![Value {
            value: UntaggedValue::row(IndexMap::default()),
            tag: raw_args.call_info.name_tag,
        }]);

        let mut idx = 0;

        while let Some(value) = input.next().await {
            let other = table.get(idx);

            match other {
                Some(replacement) => {
                    match merge_values(&value.value, &replacement.value) {
                        Ok(merged_value) => yield ReturnSuccess::value(merged_value.into_value(&value.tag)),
                        Err(err) => {
                            let message = format!("The row at {:?} types mismatch", idx);
                            yield Err(ShellError::labeled_error("Could not merge", &message, &value.tag));
                        }
                    }
                }
                None => yield ReturnSuccess::value(value),
            }

            idx += 1;
        }
    };

    Ok(stream.to_output_stream())
}
