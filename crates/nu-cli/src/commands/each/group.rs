use crate::commands::each::process_row;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    hir::Block, hir::SpannedExpression, ReturnSuccess, Scope, Signature, SyntaxShape,
    UntaggedValue, Value,
};
use nu_source::Tagged;
use serde::Deserialize;

pub struct EachGroup;

#[derive(Deserialize)]
pub struct EachGroupArgs {
    group_size: Tagged<usize>,
    block: Block,
    //numbered: Tagged<bool>,
}

#[async_trait]
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

    async fn run(
        &self,
        raw_args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let registry = registry.clone();
        let head = Arc::new(raw_args.call_info.args.head.clone());
        let scope = raw_args.call_info.scope.clone();
        let context = Arc::new(EvaluationContext::from_raw(&raw_args, &registry));
        let (each_args, input): (EachGroupArgs, _) = raw_args.process(&registry).await?;
        let block = Arc::new(each_args.block);

        Ok(input
            .chunks(each_args.group_size.item)
            .then(move |input| {
                run_block_on_vec(
                    input,
                    block.clone(),
                    scope.clone(),
                    head.clone(),
                    context.clone(),
                )
            })
            .flatten()
            .to_output_stream())
    }
}

pub(crate) fn run_block_on_vec(
    input: Vec<Value>,
    block: Arc<Block>,
    scope: Arc<Scope>,
    head: Arc<Box<SpannedExpression>>,
    context: Arc<EvaluationContext>,
) -> impl Future<Output = OutputStream> {
    let value = Value {
        value: UntaggedValue::Table(input),
        tag: Tag::unknown(),
    };

    async {
        match process_row(block, scope, head, context, value).await {
            Ok(s) => {
                // We need to handle this differently depending on whether process_row
                // returned just 1 value or if it returned multiple as a stream.
                let vec = s.collect::<Vec<_>>().await;

                // If it returned just one value, just take that value
                if vec.len() == 1 {
                    return OutputStream::one(vec.into_iter().next().expect(
                        "This should be impossible, we just checked that vec.len() == 1.",
                    ));
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
}

#[cfg(test)]
mod tests {
    use super::EachGroup;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(EachGroup {})
    }
}
