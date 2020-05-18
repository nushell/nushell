use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;

use futures::stream::once;
use nu_errors::ShellError;
use nu_protocol::{
    hir::Block, hir::Expression, hir::SpannedExpression, hir::Synthetic, ReturnSuccess, Signature,
    SyntaxShape, UntaggedValue,
};

pub struct Each;

#[derive(Deserialize)]
pub struct EachArgs {
    block: Block,
}

impl WholeStreamCommand for Each {
    fn name(&self) -> &str {
        "each"
    }

    fn signature(&self) -> Signature {
        Signature::build("each").required(
            "block",
            SyntaxShape::Block,
            "the block to run on each row",
        )
    }

    fn usage(&self) -> &str {
        "Run a block on each row of the table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        each(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Echo the square of each integer",
                example: "echo [1 2 3] | each { echo $(= $it * $it) }",
                result: Some(vec![
                    UntaggedValue::int(1).into(),
                    UntaggedValue::int(4).into(),
                    UntaggedValue::int(9).into(),
                ]),
            },
            Example {
                description: "Echo the sum of each row",
                example: "echo [[1 2] [3 4]] | each { echo $it | sum }",
                result: Some(vec![
                    UntaggedValue::int(3).into(),
                    UntaggedValue::int(7).into(),
                ]),
            },
        ]
    }
}

fn is_expanded_it_usage(head: &SpannedExpression) -> bool {
    match &*head {
        SpannedExpression {
            expr: Expression::Synthetic(Synthetic::String(s)),
            ..
        } if s == "expanded-each" => true,
        _ => false,
    }
}

fn each(raw_args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let head = raw_args.call_info.args.head.clone();
        let scope = raw_args.call_info.scope.clone();
        let mut context = Context::from_raw(&raw_args, &registry);
        let (each_args, mut input): (EachArgs, _) = raw_args.process(&registry).await?;
        let block = each_args.block;
        while let Some(input) = input.next().await {

            let input_clone = input.clone();
            let input_stream = if is_expanded_it_usage(&head) {
                InputStream::empty()
            } else {
                once(async { Ok(input) }).to_input_stream()
            };

            let result = run_block(
                &block,
                &mut context,
                input_stream,
                &scope.clone().set_it(input_clone),
            ).await;

            match result {
                Ok(mut stream) => {
                    while let Some(result) = stream.next().await {
                        yield Ok(ReturnSuccess::Value(result));
                    }

                    let errors = context.get_errors();
                    if let Some(error) = errors.first() {
                        yield Err(error.clone());
                    }
                }
                Err(e) => {
                    yield Err(e);
                }
            }
        }
    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Each;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Each {})
    }
}
