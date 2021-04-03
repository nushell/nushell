use crate::prelude::*;
use nu_engine::evaluate_baseline_expr;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    hir::CapturedBlock, hir::ClassifiedCommand, Signature, SyntaxShape, UntaggedValue,
};

pub struct Command;

#[derive(Deserialize)]
pub struct Arguments {
    block: CapturedBlock,
}

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "any?"
    }

    fn signature(&self) -> Signature {
        Signature::build("any?").required(
            "condition",
            SyntaxShape::RowCondition,
            "the condition that must match",
        )
    }

    fn usage(&self) -> &str {
        "Find if the table rows matches the condition."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        any(args).await
    }

    fn examples(&self) -> Vec<Example> {
        use nu_protocol::Value;

        vec![
            Example {
                description: "Find if a service is not running",
                example: "echo [[status]; [UP] [DOWN] [UP]] | any? status == DOWN",
                result: Some(vec![Value::from(true)]),
            },
            Example {
                description: "Check if any of the values is odd",
                example: "echo [2 4 1 6 8] | any? $(= $it mod 2) == 1",
                result: Some(vec![Value::from(true)]),
            },
        ]
    }
}

async fn any(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctx = Arc::new(EvaluationContext::from_args(&args));
    let tag = args.call_info.name_tag.clone();
    let (Arguments { block }, input) = args.process().await?;

    let condition = {
        if block.block.block.len() != 1 {
            return Err(ShellError::labeled_error(
                "Expected a condition",
                "expected a condition",
                tag,
            ));
        }
        match block.block.block[0].pipelines.get(0) {
            Some(item) => match item.list.get(0) {
                Some(ClassifiedCommand::Expr(expr)) => expr.clone(),
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
    };

    let cond = Ok(InputStream::one(
        UntaggedValue::boolean(false).into_value(&tag),
    ));

    Ok(input
        .fold(cond, move |cond, row| {
            let condition = condition.clone();
            let ctx = ctx.clone();
            ctx.scope.enter_scope();
            ctx.scope.add_vars(&block.captured.entries);
            ctx.scope.add_var("$it", row);

            async move {
                let condition = evaluate_baseline_expr(&condition, &*ctx).await.clone();
                ctx.scope.exit_scope();

                let curr = cond?.drain_vec().await;
                let curr = curr
                    .get(0)
                    .ok_or_else(|| ShellError::unexpected("No value to check with"))?;
                let cond = curr.as_bool()?;

                match condition {
                    Ok(condition) => match condition.as_bool() {
                        Ok(b) => Ok(InputStream::one(
                            UntaggedValue::boolean(cond || b).into_value(&curr.tag),
                        )),
                        Err(e) => Err(e),
                    },
                    Err(e) => Err(e),
                }
            }
        })
        .await?
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Command;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Command {})
    }
}
