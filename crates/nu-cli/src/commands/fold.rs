use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use crate::{CommandArgs, CommandRegistry, Example, OutputStream};
use futures::stream::once;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, Scope, Signature, SyntaxShape, UntaggedValue, Value};

pub struct Fold;

#[derive(Deserialize)]
pub struct FoldArgs {
    acc: Value,
    block: Block,
}

#[async_trait]
impl WholeStreamCommand for Fold {
    fn name(&self) -> &str {
        "fold"
    }

    fn signature(&self) -> Signature {
        Signature::build("fold")
            .required("start", SyntaxShape::Any, "the initial value")
            .required(
                "block",
                SyntaxShape::Block,
                "the block (function) with which to fold",
            )
    }

    fn usage(&self) -> &str {
        "Aggregate a table (TODO?) to a single value (Acc) using an accumulator block (Acc, Row -> Acc)."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        fold(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Simple summation",
                example: "echo 1 2 3 4 | fold $(= -1) { = $acc + $it }",
                result: Some(vec![UntaggedValue::int(9).into()]),
            },
            Example {
                // TODO this appears to work, but not in testing
                description: "Folding with rows",
                example: r#"echo a,b 1,2 3,4 | split column , | headers
    | fold 1.6 { = $acc * $(echo $it.a | str to-int) + $(echo $it.b | str to-int) }"#,
                result: None, // Some(vec![UntaggedValue::decimal(14.8).into()]),
            },
        ]
    }
}

async fn process_row(
    block: Arc<Block>,
    scope: Arc<Scope>,
    mut context: Arc<Context>,
    row: Value,
) -> Result<InputStream, ShellError> {
    let row_clone = row.clone();
    let input_stream = once(async { Ok(row_clone) }).to_input_stream();
    Ok(run_block(
        &block,
        Arc::make_mut(&mut context),
        input_stream,
        &row,
        &scope.vars,
        &scope.env,
    )
    .await?)
}

async fn fold(
    raw_args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let base_scope = raw_args.call_info.scope.clone();

    let context = Arc::new(Context::from_raw(&raw_args, &registry));
    let (fold_args, input): (FoldArgs, _) = raw_args.process(&registry).await?;
    let block = Arc::new(fold_args.block);

    // if fold_args.numbered.item {
    //     Ok(input
    //         .enumerate()
    //         .then(move |input| {
    //             let block = block.clone();
    //             let scope = scope.clone();
    //             let head = head.clone();
    //             let context = context.clone();
    //
    // let mut dict = TaggedDictBuilder::new(input.1.tag());
    //             dict.insert_untagged("index", UntaggedValue::int(input.0));
    //             dict.insert_value("item", input.1);
    //
    //             async {
    //                 match process_row(block, scope, head, context, dict.into_value()).await {
    //                     Ok(s) => s,
    //                     Err(e) => OutputStream::one(Err(e)),
    //                 }
    //             }
    //         })
    //         .flatten()
    //         .to_output_stream())
    // } else {
    Ok(input
        .fold(
            Ok(InputStream::one(fold_args.acc.clone())),
            move |acc, row| {
                let block = Arc::clone(&block);
                let mut scope = base_scope.clone();
                let context = Arc::clone(&context);

                async {
                    scope
                        .vars
                        .insert(String::from("$acc"), acc?.into_vec().await[0].clone());
                    process_row(block, Arc::new(scope), context, row).await
                }
            },
        )
        .await?
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Fold;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Fold {})
    }
}
