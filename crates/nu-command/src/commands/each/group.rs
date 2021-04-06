use crate::commands::each::process_row;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    hir::CapturedBlock, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tagged;
use serde::Deserialize;

pub struct EachGroup;

#[derive(Deserialize)]
pub struct EachGroupArgs {
    group_size: Tagged<usize>,
    block: CapturedBlock,
    //numbered: Tagged<bool>,
}

impl WholeStreamCommand for EachGroup {
    fn name(&self) -> &str {
        "each group"
    }

    fn signature(&self) -> Signature {
        Signature::build("each group")
            .required("group_size", SyntaxShape::Int, "the size of each group")
            .required(
                "block",
                SyntaxShape::Block,
                "the block to run on each group",
            )
    }

    fn usage(&self) -> &str {
        "Runs a block on groups of `group_size` rows of a table at a time."
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Echo the sum of each pair",
            example: "echo [1 2 3 4] | each group 2 { echo $it | math sum }",
            result: None,
        }]
    }

    fn run(&self, raw_args: CommandArgs) -> Result<OutputStream, ShellError> {
        let context = Arc::new(EvaluationContext::from_args(&raw_args));
        let (each_args, input): (EachGroupArgs, _) = raw_args.process()?;
        let block = Arc::new(Box::new(each_args.block));

        let each_group_iterator = EachGroupIterator {
            block,
            context,
            group_size: each_args.group_size.item,
            input,
        };

        Ok(each_group_iterator.flatten().to_output_stream())
    }
}

struct EachGroupIterator {
    block: Arc<Box<CapturedBlock>>,
    context: Arc<EvaluationContext>,
    group_size: usize,
    input: InputStream,
}

impl Iterator for EachGroupIterator {
    type Item = OutputStream;

    fn next(&mut self) -> Option<Self::Item> {
        let mut group = vec![];
        let mut current_count = 0;

        while let Some(next) = self.input.next() {
            group.push(next);

            current_count += 1;
            if current_count >= self.group_size {
                break;
            }
        }

        if group.is_empty() {
            return None;
        }

        Some(run_block_on_vec(
            group,
            self.block.clone(),
            self.context.clone(),
        ))
    }
}

pub(crate) fn run_block_on_vec(
    input: Vec<Value>,
    block: Arc<Box<CapturedBlock>>,
    context: Arc<EvaluationContext>,
) -> OutputStream {
    let value = Value {
        value: UntaggedValue::Table(input),
        tag: Tag::unknown(),
    };

    match process_row(block, context, value) {
        Ok(s) => {
            // We need to handle this differently depending on whether process_row
            // returned just 1 value or if it returned multiple as a stream.
            let vec = s.collect::<Vec<_>>();

            // If it returned just one value, just take that value
            if vec.len() == 1 {
                return OutputStream::one(
                    vec.into_iter()
                        .next()
                        .expect("This should be impossible, we just checked that vec.len() == 1."),
                );
            }

            // If it returned multiple values, we need to put them into a table and
            // return that.
            let result = vec.into_iter().collect::<Result<Vec<ReturnSuccess>, _>>();
            let result_table = match result {
                Ok(t) => t,
                Err(e) => return OutputStream::one(Err(e)),
            };

            let table = result_table
                .into_iter()
                .filter_map(|x| x.raw_value())
                .collect();

            OutputStream::one(Ok(ReturnSuccess::Value(UntaggedValue::Table(table).into())))
        }
        Err(e) => OutputStream::one(Err(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::EachGroup;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(EachGroup {})
    }
}
