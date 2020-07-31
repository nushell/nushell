// use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
// use crate::context::CommandRegistry;
use crate::prelude::*;
//
use futures::stream::once;
// use nu_errors::ShellError;
use crate::commands::classified::block::run_block;
use crate::{CommandArgs, CommandRegistry, Example, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{hir::Block, Primitive, ReturnSuccess, Scope, Signature, SyntaxShape, Value};
use serde_ini::de::Trait;
// , hir::Expression, hir::SpannedExpression, hir::Synthetic, Scope, ,
// , TaggedDictBuilder, , Value,
// };
// use nu_source::Tagged;

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
            // TODO unit?
            // TODO make this optional?
            .required("start", SyntaxShape::Any, "the initial value")
            .required(
                "block",
                SyntaxShape::Block,
                "the block (function) with which to fold",
            )
    }

    fn usage(&self) -> &str {
        "Reduce a table to a single value with an accumulator block." // TODO?
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
            // Example {
            //     description: "Echo the sum of each row",
            //     example: "echo [[1 2] [3 4]] | each { echo $it | math sum }",
            //     result: None,
            // },
            // Example {
            //     description: "Echo the square of each integer",
            //     example: "echo [1 2 3] | each { echo $(= $it * $it) }",
            //     result: Some(vec![
            //         UntaggedValue::int(1).into(),
            //         UntaggedValue::int(4).into(),
            //         UntaggedValue::int(9).into(),
            //     ]),
            // },
            // Example {
            //     description: "Number each item and echo a message",
            //     example:
            //         "echo ['bob' 'fred'] | each --numbered { echo `{{$it.index}} is {{$it.item}}` }",
            //     result: Some(vec![Value::from("0 is bob"), Value::from("1 is fred")]),
            // },
        ]
    }
}

// fn is_expanded_it_usage(head: &SpannedExpression) -> bool {
//     matches!(&*head, SpannedExpression {
//         expr: Expression::Synthetic(Synthetic::String(s)),
//         ..
//     } if s == "expanded-each")
// }

async fn process_row(
    block: Arc<Block>,
    scope: Arc<Scope>,
    // head: Arc<Box<SpannedExpression>>,
    mut context: Arc<Context>,
    row: Value,
) -> Result<InputStream, ShellError> {
    let row_clone = row.clone();
    let input_stream = once(async { Ok(row_clone) }).to_input_stream();
    //     if is_expanded_it_usage(&head) {
    //     InputStream::empty()
    // } else {
    // };
    // println!("{:#?}", scope);
    // println!(
    //     "{:#?}",
    Ok(run_block(
        &block,
        Arc::make_mut(&mut context),
        input_stream,
        &row,
        &scope.vars,
        &scope.env,
    )
    .await?) // .into_vec()
             // .await
             // .to_output_stream()
             // );
             // Err(ShellError::unimplemented("foo"))
}

async fn fold(
    raw_args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    // let head = Arc::new(raw_args.call_info.args.head.clone());
    // let mut scope = Arc::new(raw_args.call_info.scope.clone());
    let base_scope = raw_args.call_info.scope.clone();

    let context = Arc::new(Context::from_raw(&raw_args, &registry));
    let (fold_args, input): (FoldArgs, _) = raw_args.process(&registry).await?;
    let block = Arc::new(fold_args.block);

    // let mut counter = 0;
    // base_scope
    //     .vars
    //     .insert(String::from("$acc"), fold_args.acc.clone());

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
    // echo 1 2 | fold 0 { = $acc + $it }
    Ok(input
        .fold(
            Ok(InputStream::one(fold_args.acc.clone())),
            move |acc, row| {
                let block = Arc::clone(&block); // TODO Arc::clone(block)?
                let mut scope = base_scope.clone();
                let context = Arc::clone(&context);

                // let head = head.clone();

                async {
                    // println!("{:#?}", &acc.unwrap().into_vec().await[0]);
                    scope
                        .vars
                        .insert(String::from("$acc"), acc?.into_vec().await[0].clone());
                    process_row(block, Arc::new(scope), context, row).await
                }
            },
        )
        .await?
        .to_output_stream())
    // .then(move |row| {
    //
    // })
    // .flatten()
    // );
    // }
    // Err(ShellError::unimplemented("fold"))
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
