use crate::prelude::*;
use nu_data::value::merge_values;
use nu_engine::run_block;
use nu_engine::WholeStreamCommand;

use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{
    hir::CapturedBlock, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
pub struct Merge;

#[derive(Deserialize)]
pub struct MergeArgs {
    block: CapturedBlock,
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        merge(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Merge a 1-based index column with some ls output",
            example: "ls | select name | keep 3 | merge { echo [1 2 3] | wrap index }",
            result: None,
        }]
    }
}

async fn merge(raw_args: CommandArgs) -> Result<OutputStream, ShellError> {
    let context = EvaluationContext::from_args(&raw_args);
    let name_tag = raw_args.call_info.name_tag.clone();
    let (merge_args, input): (MergeArgs, _) = raw_args.process().await?;
    let block = merge_args.block;

    context.scope.enter_scope();
    context.scope.add_vars(&block.captured.entries);
    let result = run_block(&block.block, &context, InputStream::empty()).await;
    context.scope.exit_scope();

    let table: Option<Vec<Value>> = match result {
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
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Merge {})
    }
}
