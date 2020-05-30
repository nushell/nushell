use crate::commands::WholeStreamCommand;
use crate::evaluate::evaluate_baseline_expr;
use crate::prelude::*;
use log::trace;
use nu_errors::ShellError;
use nu_protocol::{
    hir::ClassifiedCommand, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};

pub struct SkipUntil;

#[async_trait]
impl WholeStreamCommand for SkipUntil {
    fn name(&self) -> &str {
        "skip-until"
    }

    fn signature(&self) -> Signature {
        Signature::build("skip-until")
            .required(
                "condition",
                SyntaxShape::Math,
                "the condition that must be met to stop skipping",
            )
            .filter()
    }

    fn usage(&self) -> &str {
        "Skips rows until the condition matches."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let registry = registry.clone();
        let scope = args.call_info.scope.clone();
        let stream = async_stream! {
            let mut call_info = args.evaluate_once(&registry).await?;

            let block = call_info.args.expect_nth(0)?.clone();

            let condition = match block {
                Value {
                    value: UntaggedValue::Block(block),
                    tag,
                } => {
                    if block.block.len() != 1 {
                        yield Err(ShellError::labeled_error(
                            "Expected a condition",
                            "expected a condition",
                            tag,
                        ));
                        return;
                    }
                    match block.block[0].list.get(0) {
                        Some(item) => match item {
                            ClassifiedCommand::Expr(expr) => expr.clone(),
                            _ => {
                                yield Err(ShellError::labeled_error(
                                    "Expected a condition",
                                    "expected a condition",
                                    tag,
                                ));
                                return;
                            }
                        },
                        None => {
                            yield Err(ShellError::labeled_error(
                                "Expected a condition",
                                "expected a condition",
                                tag,
                            ));
                            return;
                        }
                    }
                }
                Value { tag, .. } => {
                    yield Err(ShellError::labeled_error(
                        "Expected a condition",
                        "expected a condition",
                        tag,
                    ));
                    return;
                }
            };

            let mut skipping = true;
            while let Some(item) = call_info.input.next().await {
                let condition = condition.clone();
                trace!("ITEM = {:?}", item);
                let result =
                    evaluate_baseline_expr(&*condition, &registry, &item, &scope.vars, &scope.env)
                        .await;
                trace!("RESULT = {:?}", result);

                let return_value = match result {
                    Ok(ref v) if v.is_true() => true,
                    _ => false,
                };

                if return_value {
                    skipping = false;
                }

                if !skipping {
                    yield ReturnSuccess::value(item);
                }
            }
        };

        Ok(stream.to_output_stream())
    }
}

#[cfg(test)]
mod tests {
    use super::SkipUntil;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SkipUntil {})
    }
}
