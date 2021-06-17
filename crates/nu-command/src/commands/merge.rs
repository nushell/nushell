use crate::prelude::*;
use nu_data::value::merge_values;
use nu_engine::run_block;
use nu_engine::WholeStreamCommand;

use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{
    hir::CapturedBlock, hir::ExternalRedirection, ReturnSuccess, Signature, SyntaxShape,
    UntaggedValue, Value,
};
pub struct Merge;

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

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        merge(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Merge a 1-based index column with some ls output",
            example: "ls | select name | keep 3 | merge { echo [1 2 3] | wrap index }",
            result: None,
        }]
    }
}

fn merge(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let context = &args.context;
    let name_tag = args.call_info.name_tag.clone();

    let block: CapturedBlock = args.req(0)?;
    let input = args.input;

    context.scope.enter_scope();
    context.scope.add_vars(&block.captured.entries);
    let result = run_block(
        &block.block,
        &context,
        InputStream::empty(),
        ExternalRedirection::Stdout,
    );
    context.scope.exit_scope();

    let table: Option<Vec<Value>> = match result {
        Ok(mut stream) => Some(stream.drain_vec()),
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
        .into_action_stream())
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
