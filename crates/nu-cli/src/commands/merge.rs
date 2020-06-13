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

#[async_trait]
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        merge(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Merge a 1-based index column with some ls output",
            example: "ls | select name | keep 3 | merge { echo [1 2 3] | wrap index }",
            result: None,
        }]
    }
}

async fn merge(
    raw_args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let scope = raw_args.call_info.scope.clone();
    let mut context = Context::from_raw(&raw_args, &registry);
    let name_tag = raw_args.call_info.name_tag.clone();
    let (merge_args, input): (MergeArgs, _) = raw_args.process(&registry).await?;
    let block = merge_args.block;

    let table: Option<Vec<Value>> = match run_block(
        &block,
        &mut context,
        InputStream::empty(),
        &scope.it,
        &scope.vars,
        &scope.env,
    )
    .await
    {
        Ok(mut stream) => Some(stream.drain_vec().await),
        Err(err) => {
            return Err(err);
        }
    };

    let table = table.unwrap_or_else(|| {
        vec![Value {
            value: UntaggedValue::row(IndexMap::default()),
            tag: name_tag,
        }]
    });

    Ok(input
        .enumerate()
        .map(move |(idx, value)| {
            let other = table.get(idx);

            match other {
                Some(replacement) => match merge_values(&value.value, &replacement.value) {
                    Ok(merged_value) => ReturnSuccess::value(merged_value.into_value(&value.tag)),
                    Err(_) => {
                        let message = format!("The row at {:?} types mismatch", idx);
                        Err(ShellError::labeled_error(
                            "Could not merge",
                            &message,
                            &value.tag,
                        ))
                    }
                },
                None => ReturnSuccess::value(value),
            }
        })
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Merge;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Merge {})
    }
}
