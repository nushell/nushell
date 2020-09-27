use crate::command_registry::CommandRegistry;
use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;

use nu_errors::ShellError;
use nu_protocol::{
    hir::Block, hir::Expression, hir::SpannedExpression, hir::Synthetic, Scope, Signature,
    SyntaxShape,
};

pub struct OnlyOne;

#[derive(Deserialize)]
pub struct OnlyOneArgs {
    block: Block,
}

#[async_trait]
impl WholeStreamCommand for OnlyOne {
    fn name(&self) -> &str {
        "only-one"
    }

    fn signature(&self) -> Signature {
        Signature::build("only one").required(
            "block",
            SyntaxShape::Block,
            "the block to run on the first value in the stream",
        )
    }

    fn usage(&self) -> &str {
        "Get only the first value in the stream and run block, passing the value as $it."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        only_one(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

fn is_expanded_only_one_usage(head: &SpannedExpression) -> bool {
    matches!(&*head, SpannedExpression {
        expr: Expression::Synthetic(Synthetic::String(s)),
        ..
    } if s == "expanded-only-one")
}

async fn only_one(
    raw_args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let head = Arc::new(raw_args.call_info.args.head.clone());
    let scope = raw_args.call_info.scope.clone();
    let mut context = EvaluationContext::from_raw(&raw_args, &registry);
    let (only_on_args, mut input): (OnlyOneArgs, InputStream) = raw_args.process(&registry).await?;
    let block = Arc::new(only_on_args.block);

    let val = input.next().await;

    match val {
        Some(value) => {
            let input_stream = if is_expanded_only_one_usage(&head) {
                InputStream::empty()
            } else {
                InputStream::one(value.clone())
            };

            let scope = Scope::append_var(scope, "$it".into(), value);

            Ok(run_block(&block, &mut context, input_stream, scope)
                .await?
                .to_output_stream())
        }
        None => Ok(OutputStream::empty()),
    }
}

#[cfg(test)]
mod tests {
    use super::OnlyOne;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(OnlyOne {})
    }
}
