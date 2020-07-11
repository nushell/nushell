use crate::commands::classified::block::run_block;
use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::evaluate::evaluate_baseline_expr;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{hir::Block, hir::ClassifiedCommand, Signature, SyntaxShape, UntaggedValue};

pub struct If;

#[derive(Deserialize)]
pub struct IfArgs {
    condition: Block,
    then_case: Block,
    else_case: Block,
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
                SyntaxShape::Math,
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
        "Filter table to match the condition."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        if_command(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Run a block if a condition is true",
                example: "echo 10 | if $it > 5 { echo 'greater than 5' } { echo 'less than or equal to 5' }",
                result: Some(vec![UntaggedValue::string("greater than 5").into()]),
            },
            Example {
                description: "Run a block if a condition is false",
                example: "echo 1 | if $it > 5 { echo 'greater than 5' } { echo 'less than or equal to 5' }",
                result: Some(vec![UntaggedValue::string("less than or equal to 5").into()]),
            },
        ]
    }
}
async fn if_command(
    raw_args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = Arc::new(registry.clone());
    let scope = Arc::new(raw_args.call_info.scope.clone());
    let tag = raw_args.call_info.name_tag.clone();
    let context = Arc::new(Context::from_raw(&raw_args, &registry));

    let (
        IfArgs {
            condition,
            then_case,
            else_case,
        },
        input,
    ) = raw_args.process(&registry).await?;
    let condition = {
        if condition.block.len() != 1 {
            return Err(ShellError::labeled_error(
                "Expected a condition",
                "expected a condition",
                tag,
            ));
        }
        match condition.block[0].list.get(0) {
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
        .then(move |input| {
            let condition = condition.clone();
            let then_case = then_case.clone();
            let else_case = else_case.clone();
            let registry = registry.clone();
            let scope = scope.clone();
            let mut context = context.clone();

            async move {
                //FIXME: should we use the scope that's brought in as well?
                let condition =
                    evaluate_baseline_expr(&condition, &*registry, &input, &scope.vars, &scope.env)
                        .await;

                match condition {
                    Ok(condition) => match condition.as_bool() {
                        Ok(b) => {
                            if b {
                                match run_block(
                                    &then_case,
                                    Arc::make_mut(&mut context),
                                    InputStream::empty(),
                                    &input,
                                    &scope.vars,
                                    &scope.env,
                                )
                                .await
                                {
                                    Ok(stream) => stream.to_output_stream(),
                                    Err(e) => futures::stream::iter(vec![Err(e)].into_iter())
                                        .to_output_stream(),
                                }
                            } else {
                                match run_block(
                                    &else_case,
                                    Arc::make_mut(&mut context),
                                    InputStream::empty(),
                                    &input,
                                    &scope.vars,
                                    &scope.env,
                                )
                                .await
                                {
                                    Ok(stream) => stream.to_output_stream(),
                                    Err(e) => futures::stream::iter(vec![Err(e)].into_iter())
                                        .to_output_stream(),
                                }
                            }
                        }
                        Err(e) => {
                            futures::stream::iter(vec![Err(e)].into_iter()).to_output_stream()
                        }
                    },
                    Err(e) => futures::stream::iter(vec![Err(e)].into_iter()).to_output_stream(),
                }
            }
        })
        .flatten()
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::If;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(If {})
    }
}
