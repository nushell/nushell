use crate::commands::classified::block::run_block;
use crate::commands::each;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use crate::{CommandArgs, CommandRegistry, Example, OutputStream};
use futures::stream::once;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, Primitive, Scope, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;

pub struct Reduce;

#[derive(Deserialize)]
pub struct ReduceArgs {
    block: Block,
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        reduce(args, registry).await
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

async fn reduce(
    raw_args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let base_scope = raw_args.call_info.scope.clone();
    let context = Arc::new(Context::from_raw(&raw_args, &registry));
    let (reduce_args, mut input): (ReduceArgs, _) = raw_args.process(&registry).await?;
    let block = Arc::new(reduce_args.block);
    let (ioffset, start) = match reduce_args.fold {
        None => {
            let first = input
                .next()
                .await
                .expect("empty stream expected to contain Primitive::Nothing");
            if let UntaggedValue::Primitive(Primitive::Nothing) = first.value {
                return Err(ShellError::missing_value(None, "empty input"));
            }

            (1, first)
        }
        Some(acc) => (0, acc),
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
                let block = Arc::clone(&block);
                let mut scope = base_scope.clone();
                let context = Arc::clone(&context);
                let row = each::make_indexed_item(input.0 + ioffset, input.1);

                async {
                    let f = acc?.into_vec().await[0].clone();
                    scope.vars.insert(String::from("$acc"), f);
                    process_row(block, Arc::new(scope), context, row).await
                }
            })
            .await?
            .to_output_stream())
    } else {
        let initial = Ok(InputStream::one(start));
        Ok(input
            .fold(initial, move |acc, row| {
                let block = Arc::clone(&block);
                let mut scope = base_scope.clone();
                let context = Arc::clone(&context);

                async {
                    scope
                        .vars
                        .insert(String::from("$acc"), acc?.into_vec().await[0].clone());
                    process_row(block, Arc::new(scope), context, row).await
                }
            })
            .await?
            .to_output_stream())
    }
}

#[cfg(test)]
mod tests {
    use super::Reduce;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Reduce {})
    }
}
