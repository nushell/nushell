use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::evaluate::evaluate_baseline_expr;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, hir::ClassifiedCommand, ReturnSuccess, Signature, SyntaxShape};

pub struct Where;

#[derive(Deserialize)]
pub struct WhereArgs {
    block: Block,
}

#[async_trait]
impl WholeStreamCommand for Where {
    fn name(&self) -> &str {
        "where"
    }

    fn signature(&self) -> Signature {
        Signature::build("where").required(
            "condition",
            SyntaxShape::Math,
            "the condition that must match",
        )
    }

    fn usage(&self) -> &str {
        "Filter table to match the condition."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        where_command(args, registry).await
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
                example: "ls | where modified <= 2M",
                result: None,
            },
        ]
    }
}
async fn where_command(
    raw_args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = Arc::new(registry.clone());
    let scope = Arc::new(raw_args.call_info.scope.clone());
    let tag = raw_args.call_info.name_tag.clone();
    let (WhereArgs { block }, input) = raw_args.process(&registry).await?;
    let condition = {
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
    };

    Ok(input
        .filter_map(move |input| {
            let condition = condition.clone();
            let registry = registry.clone();
            let scope = scope.clone();

            async move {
                //FIXME: should we use the scope that's brought in as well?
                let condition =
                    evaluate_baseline_expr(&condition, &*registry, &input, &scope.vars, &scope.env)
                        .await;

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
    use super::Where;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Where {})
    }
}
