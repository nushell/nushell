use crate::commands::WholeStreamCommand;
use crate::evaluate::evaluate_baseline_expr;
use crate::prelude::*;
use log::trace;
use nu_errors::ShellError;
use nu_protocol::{hir::ClassifiedCommand, Scope, Signature, SyntaxShape, UntaggedValue, Value};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "keep until"
    }

    fn signature(&self) -> Signature {
        Signature::build("keep until")
            .required(
                "condition",
                SyntaxShape::Math,
                "The condition that must be met to stop keeping rows",
            )
            .filter()
    }

    fn usage(&self) -> &str {
        "Keeps rows until the condition matches."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let registry = Arc::new(registry.clone());
        let scope = args.call_info.scope.clone();

        let call_info = args.evaluate_once(&registry).await?;

        let block = call_info.args.expect_nth(0)?.clone();

        let condition = Arc::new(match block {
            Value {
                value: UntaggedValue::Block(block),
                tag,
            } => {
                if block.block.len() != 1 {
                    return Err(ShellError::labeled_error(
                        "Expected a condition",
                        "expected a condition",
                        tag,
                    ));
                }
                match block.block[0].list.get(0) {
                    Some(item) => match item {
                        ClassifiedCommand::Expr(expr) => expr.clone(),
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
        });

        Ok(call_info
            .input
            .take_while(move |item| {
                let condition = condition.clone();
                let registry = registry.clone();
                let scope = Scope::append_it(scope.clone(), item.clone());
                trace!("ITEM = {:?}", item);

                async move {
                    let result = evaluate_baseline_expr(&*condition, &registry, scope).await;
                    trace!("RESULT = {:?}", result);

                    !matches!(result, Ok(ref v) if v.is_true())
                }
            })
            .to_output_stream())
    }
}

#[cfg(test)]
mod tests {
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
