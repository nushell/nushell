use crate::prelude::*;
use nu_engine::evaluate_baseline_expr;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    hir::CapturedBlock, hir::ClassifiedCommand, ReturnSuccess, Signature, SyntaxShape,
};

pub struct Where;

#[derive(Deserialize)]
pub struct WhereArgs {
    block: CapturedBlock,
}

#[async_trait]
impl WholeStreamCommand for Where {
    fn name(&self) -> &str {
        "where"
    }

    fn signature(&self) -> Signature {
        Signature::build("where").required(
            "condition",
            SyntaxShape::RowCondition,
            "the condition that must match",
        )
    }

    fn usage(&self) -> &str {
        "Filter table to match the condition."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        where_command(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "List all files in the current directory with sizes greater than 2kb",
                example: "ls | where size > 2kb",
                result: None,
            },
            Example {
                description: "List only the files in the current directory",
                example: "ls | where type == File",
                result: None,
            },
            Example {
                description: "List all files with names that contain \"Car\"",
                example: "ls | where name =~ \"Car\"",
                result: None,
            },
            Example {
                description: "List all files that were modified in the last two months",
                example: "ls | where modified <= 2mon",
                result: None,
            },
        ]
    }
}
async fn where_command(raw_args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctx = Arc::new(EvaluationContext::from_args(&raw_args));
    let tag = raw_args.call_info.name_tag.clone();
    let (WhereArgs { block }, input) = raw_args.process().await?;
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

    Ok(input
        .filter_map(move |input| {
            let condition = condition.clone();
            let ctx = ctx.clone();

            ctx.scope.enter_scope();
            ctx.scope.add_vars(&block.captured.entries);
            ctx.scope.add_var("$it", input.clone());

            async move {
                //FIXME: should we use the scope that's brought in as well?
                let condition = evaluate_baseline_expr(&condition, &*ctx).await;
                ctx.scope.exit_scope();

                match condition {
                    Ok(condition) => match condition.as_bool() {
                        Ok(b) => {
                            if b {
                                Some(Ok(ReturnSuccess::Value(input)))
                            } else {
                                None
                            }
                        }
                        Err(e) => Some(Err(e)),
                    },
                    Err(e) => Some(Err(e)),
                }
            }
        })
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::Where;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Where {})
    }
}
