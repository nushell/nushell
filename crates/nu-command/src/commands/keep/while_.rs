use crate::prelude::*;
use log::trace;
use nu_engine::evaluate_baseline_expr;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{hir::ClassifiedCommand, Signature, SyntaxShape, UntaggedValue, Value};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "keep while"
    }

    fn signature(&self) -> Signature {
        Signature::build("keep while")
            .required(
                "condition",
                SyntaxShape::RowCondition,
                "The condition that must be met to keep rows",
            )
            .filter()
    }

    fn usage(&self) -> &str {
        "Keeps rows while the condition matches."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let ctx = Arc::new(EvaluationContext::from_args(&args));
        let call_info = args.evaluate_once().await?;

        let block = call_info.args.expect_nth(0)?.clone();

        let (condition, captured) = match block {
            Value {
                value: UntaggedValue::Block(captured_block),
                tag,
            } => {
                if captured_block.block.block.len() != 1 {
                    return Err(ShellError::labeled_error(
                        "Expected a condition",
                        "expected a condition",
                        tag,
                    ));
                }
                match captured_block.block.block[0].pipelines.get(0) {
                    Some(item) => match item.list.get(0) {
                        Some(ClassifiedCommand::Expr(expr)) => {
                            (Arc::new(expr.clone()), captured_block.captured.clone())
                        }
                        _ => {
                            return Err(ShellError::labeled_error(
                                "Expected a condition",
                                "expected a condition",
                                tag,
                            ));
                        }
                    },
                    None => {
                        return Err(ShellError::labeled_error(
                            "Expected a condition",
                            "expected a condition",
                            tag,
                        ));
                    }
                }
            }
            Value { tag, .. } => {
                return Err(ShellError::labeled_error(
                    "Expected a condition",
                    "expected a condition",
                    tag,
                ));
            }
        };

        Ok(call_info
            .input
            .take_while(move |item| {
                let condition = condition.clone();
                let ctx = ctx.clone();

                ctx.scope.enter_scope();
                ctx.scope.add_var("$it", item.clone());
                ctx.scope.add_vars(&captured.entries);
                trace!("ITEM = {:?}", item);

                async move {
                    let result = evaluate_baseline_expr(&*condition, &*ctx).await;
                    ctx.scope.exit_scope();
                    trace!("RESULT = {:?}", result);

                    matches!(result, Ok(ref v) if v.is_true())
                }
            })
            .to_output_stream())
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
