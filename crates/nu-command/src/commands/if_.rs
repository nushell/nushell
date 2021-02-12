use crate::prelude::*;
use nu_engine::evaluate_baseline_expr;
use nu_engine::run_block;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    hir::CapturedBlock, hir::ClassifiedCommand, Signature, SyntaxShape, UntaggedValue,
};

pub struct If;

#[derive(Deserialize)]
pub struct IfArgs {
    condition: CapturedBlock,
    then_case: CapturedBlock,
    else_case: CapturedBlock,
}

#[async_trait]
impl WholeStreamCommand for If {
    fn name(&self) -> &str {
        "if"
    }

    fn signature(&self) -> Signature {
        Signature::build("if")
            .required(
                "condition",
                SyntaxShape::MathExpression,
                "the condition that must match",
            )
            .required(
                "then_case",
                SyntaxShape::Block,
                "block to run if condition is true",
            )
            .required(
                "else_case",
                SyntaxShape::Block,
                "block to run if condition is false",
            )
    }

    fn usage(&self) -> &str {
        "Run blocks if a condition is true or false."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        if_command(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Run a block if a condition is true",
                example: "let x = 10; if $x > 5 { echo 'greater than 5' } { echo 'less than or equal to 5' }",
                result: Some(vec![UntaggedValue::string("greater than 5").into()]),
            },
            Example {
                description: "Run a block if a condition is false",
                example: "let x = 1; if $x > 5 { echo 'greater than 5' } { echo 'less than or equal to 5' }",
                result: Some(vec![UntaggedValue::string("less than or equal to 5").into()]),
            },
        ]
    }
}
async fn if_command(raw_args: CommandArgs) -> Result<OutputStream, ShellError> {
    let tag = raw_args.call_info.name_tag.clone();
    let context = Arc::new(EvaluationContext::from_args(&raw_args));

    let (
        IfArgs {
            condition,
            then_case,
            else_case,
        },
        input,
    ) = raw_args.process().await?;
    let cond = {
        if condition.block.block.len() != 1 {
            return Err(ShellError::labeled_error(
                "Expected a condition",
                "expected a condition",
                tag,
            ));
        }
        match condition.block.block[0].pipelines.get(0) {
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

    context.scope.enter_scope();
    context.scope.add_vars(&condition.captured.entries);

    //FIXME: should we use the scope that's brought in as well?
    let condition = evaluate_baseline_expr(&cond, &*context).await;
    match condition {
        Ok(condition) => match condition.as_bool() {
            Ok(b) => {
                let result = if b {
                    run_block(&then_case.block, &*context, input).await
                } else {
                    run_block(&else_case.block, &*context, input).await
                };
                context.scope.exit_scope();

                result.map(|x| x.to_output_stream())
            }
            Err(e) => Ok(futures::stream::iter(vec![Err(e)].into_iter()).to_output_stream()),
        },
        Err(e) => Ok(futures::stream::iter(vec![Err(e)].into_iter()).to_output_stream()),
    }
}

#[cfg(test)]
mod tests {
    use super::If;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(If {})
    }
}
