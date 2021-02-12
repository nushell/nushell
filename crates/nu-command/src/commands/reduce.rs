use crate::commands::each;
use crate::prelude::*;
use futures::stream::once;
use nu_engine::run_block;
use nu_engine::WholeStreamCommand;
use nu_engine::{CommandArgs, Example};
use nu_errors::ShellError;
use nu_parser::ParserScope;
use nu_protocol::{hir::CapturedBlock, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use nu_stream::OutputStream;

pub struct Reduce;

#[derive(Deserialize)]
pub struct ReduceArgs {
    block: CapturedBlock,
    fold: Option<Value>,
    numbered: Tagged<bool>,
}

#[async_trait]
impl WholeStreamCommand for Reduce {
    fn name(&self) -> &str {
        "reduce"
    }

    fn signature(&self) -> Signature {
        Signature::build("reduce")
            .named(
                "fold",
                SyntaxShape::Any,
                "reduce with initial value",
                Some('f'),
            )
            .required("block", SyntaxShape::Block, "reducing function")
            .switch(
                "numbered",
                "returned a numbered item ($it.index and $it.item)",
                Some('n'),
            )
    }

    fn usage(&self) -> &str {
        "Aggregate a list table to a single value using an accumulator block. Block must be
        (A, A) -> A unless --fold is selected, in which case it may be A, B -> A."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        reduce(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Simple summation (equivalent to math sum)",
                example: "echo 1 2 3 4 | reduce { = $acc + $it }",
                result: Some(vec![UntaggedValue::int(10).into()]),
            },
            Example {
                description: "Summation from starting value using fold",
                example: "echo 1 2 3 4 | reduce -f $(= -1) { = $acc + $it }",
                result: Some(vec![UntaggedValue::int(9).into()]),
            },
            Example {
                description: "Folding with rows",
                example: "<table> | reduce -f 1.6 { = $acc * $(echo $it.a | str to-int) + $(echo $it.b | str to-int) }",
                result: None,
            },
            Example {
                description: "Numbered reduce to find index of longest word",
                example: "echo one longest three bar | reduce -n { if $(echo $it.item | str length) > $(echo $acc.item | str length) {echo $it} {echo $acc}} | get index",
                result: None,
            },
        ]
    }
}

async fn process_row(
    block: Arc<CapturedBlock>,
    context: &EvaluationContext,
    row: Value,
) -> Result<InputStream, ShellError> {
    let row_clone = row.clone();
    let input_stream = once(async { Ok(row_clone) }).to_input_stream();

    context.scope.enter_scope();
    context.scope.add_vars(&block.captured.entries);
    context.scope.add_var("$it", row);
    let result = run_block(&block.block, context, input_stream).await;
    context.scope.exit_scope();

    Ok(result?)
}

async fn reduce(raw_args: CommandArgs) -> Result<OutputStream, ShellError> {
    let span = raw_args.call_info.name_tag.span;
    let context = Arc::new(EvaluationContext::from_args(&raw_args));
    let (reduce_args, mut input): (ReduceArgs, _) = raw_args.process().await?;
    let block = Arc::new(reduce_args.block);
    let (ioffset, start) = if !input.is_empty() {
        match reduce_args.fold {
            None => {
                let first = input.next().await.expect("non-empty stream");

                (1, first)
            }
            Some(acc) => (0, acc),
        }
    } else {
        return Err(ShellError::labeled_error(
            "Expected input",
            "needs input",
            span,
        ));
    };

    if reduce_args.numbered.item {
        // process_row returns Result<InputStream, ShellError>, so we must fold with one
        let initial = Ok(InputStream::one(each::make_indexed_item(
            ioffset - 1,
            start,
        )));

        Ok(input
            .enumerate()
            .fold(initial, move |acc, input| {
                let context = context.clone();
                let block = Arc::clone(&block);
                let row = each::make_indexed_item(input.0 + ioffset, input.1);

                async move {
                    let values = acc?.drain_vec().await;

                    let f = if values.len() == 1 {
                        let value = values
                            .get(0)
                            .ok_or_else(|| ShellError::unexpected("No value to update with"))?;
                        value.clone()
                    } else if values.is_empty() {
                        UntaggedValue::nothing().into_untagged_value()
                    } else {
                        UntaggedValue::table(&values).into_untagged_value()
                    };

                    context.scope.enter_scope();
                    context.scope.add_var("$acc", f);
                    let result = process_row(block, &*context, row).await;
                    context.scope.exit_scope();

                    result
                }
            })
            .await?
            .to_output_stream())
    } else {
        let initial = Ok(InputStream::one(start));
        Ok(input
            .fold(initial, move |acc, row| {
                let block = Arc::clone(&block);
                let context = context.clone();

                async move {
                    let values = acc?.drain_vec().await;

                    let f = if values.len() == 1 {
                        let value = values
                            .get(0)
                            .ok_or_else(|| ShellError::unexpected("No value to update with"))?;
                        value.clone()
                    } else if values.is_empty() {
                        UntaggedValue::nothing().into_untagged_value()
                    } else {
                        UntaggedValue::table(&values).into_untagged_value()
                    };

                    context.scope.enter_scope();
                    context.scope.add_var("$acc", f);
                    let result = process_row(block, &*context, row).await;
                    context.scope.exit_scope();
                    result
                }
            })
            .await?
            .to_output_stream())
    }
}

#[cfg(test)]
mod tests {
    use super::Reduce;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Reduce {})
    }
}
