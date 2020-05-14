use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;

use futures::stream::once;
use nu_errors::ShellError;
use nu_protocol::{
    hir::Block, hir::Expression, hir::SpannedExpression, hir::Synthetic, ReturnSuccess, Signature,
    SyntaxShape,
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
        Ok(args.process_raw(registry, each)?.run())
    }

    fn examples(&self) -> &[Example] {
        &[Example {
            description: "Print the name of each file",
            example: "ls | each { echo $it.name }",
        }]
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

fn each(
    each_args: EachArgs,
    context: RunnableContext,
    raw_args: RawCommandArgs,
) -> Result<OutputStream, ShellError> {
    let block = each_args.block;
    let scope = raw_args.call_info.scope.clone();
    let registry = context.registry.clone();
    let mut input_stream = context.input;
    let stream = async_stream! {
        while let Some(input) = input_stream.next().await {
            let mut context = Context::from_raw(&raw_args, &registry);

            let input_clone = input.clone();
            let input_stream = if is_expanded_it_usage(&raw_args.call_info.args.head) {
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
