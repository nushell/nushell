use crate::command_registry::CommandRegistry;
use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;

use nu_errors::ShellError;
use nu_protocol::{
    hir::Block, hir::Expression, hir::SpannedExpression, hir::Synthetic, Scope, Signature,
    SyntaxShape, UntaggedValue,
};

pub struct Collect;

#[derive(Deserialize)]
pub struct CollectArgs {
    block: Block,
}

#[async_trait]
impl WholeStreamCommand for Collect {
    fn name(&self) -> &str {
        "collect"
    }

    fn signature(&self) -> Signature {
        Signature::build("collect").required(
            "block",
            SyntaxShape::Block,
            "the block to run on each row",
        )
    }

    fn usage(&self) -> &str {
        "Collect all input into a single table and run block, passing it $table."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        collect(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

fn is_expanded_table_usage(head: &SpannedExpression) -> bool {
    matches!(&*head, SpannedExpression {
        expr: Expression::Synthetic(Synthetic::String(s)),
        ..
    } if s == "expanded-collect")
}

async fn collect(
    raw_args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let head = Arc::new(raw_args.call_info.args.head.clone());

    let scope = raw_args.call_info.scope.clone();
    let mut context = EvaluationContext::from_raw(&raw_args, &registry);
    let (collect_args, mut input): (CollectArgs, InputStream) = raw_args.process(&registry).await?;
    let block = Arc::new(collect_args.block);

    let collected = input.drain_vec().await;

    let value = UntaggedValue::table(&collected).into_untagged_value();

    let input_stream = if is_expanded_table_usage(&head) {
        InputStream::empty()
    } else {
        InputStream::from_stream(futures::stream::iter(collected))
    };

    let scope = Scope::append_var(scope, "$table".into(), value);

    Ok(run_block(&block, &mut context, input_stream, scope)
        .await?
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Collect;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Collect {})
    }
}
