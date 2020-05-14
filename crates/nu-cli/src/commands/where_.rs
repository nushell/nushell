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

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        where_command(args, registry)
    }
}
fn where_command(
    raw_args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let tag = raw_args.call_info.name_tag.clone();
        let (WhereArgs { block }, mut input) = raw_args.process(&registry).await?;
        let condition = {
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
        };

        let mut input = input;
        let scope = raw_args.call_info.scope;
        while let Some(input) = input.next().await {

            //FIXME: should we use the scope that's brought in as well?
            let condition = evaluate_baseline_expr(&condition, &registry, &scope.clone().set_it(input.clone())).await?;

            match condition.as_bool() {
                Ok(b) => {
                    if b {
                        yield Ok(ReturnSuccess::Value(input));
                    }
                }
                Err(e) => yield Err(e),
            };
        }
    };

    Ok(stream.to_output_stream())
}
