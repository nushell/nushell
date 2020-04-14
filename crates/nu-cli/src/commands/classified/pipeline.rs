use crate::commands::classified::expr::run_expression_block;
use crate::commands::classified::external::run_external_command;
use crate::commands::classified::internal::run_internal_command;
use crate::context::Context;
use crate::stream::InputStream;
use nu_errors::ShellError;
use nu_protocol::hir::{ClassifiedCommand, ClassifiedPipeline};
use nu_protocol::Scope;

pub(crate) async fn run_pipeline(
    pipeline: ClassifiedPipeline,
    ctx: &mut Context,
    mut input: Option<InputStream>,
    scope: &Scope,
) -> Result<Option<InputStream>, ShellError> {
    let mut iter = pipeline.commands.list.into_iter().peekable();

    loop {
        let item: Option<ClassifiedCommand> = iter.next();
        let next: Option<&ClassifiedCommand> = iter.peek();

        input = match (item, next) {
            (Some(ClassifiedCommand::Dynamic(_)), _) | (_, Some(ClassifiedCommand::Dynamic(_))) => {
                return Err(ShellError::unimplemented("Dynamic commands"))
            }

            (Some(ClassifiedCommand::Expr(expr)), _) => {
                run_expression_block(*expr, ctx, input, scope)?
            }
            (Some(ClassifiedCommand::Error(err)), _) => return Err(err.into()),
            (_, Some(ClassifiedCommand::Error(err))) => return Err(err.clone().into()),

            (Some(ClassifiedCommand::Internal(left)), _) => {
                run_internal_command(left, ctx, input, scope)?
            }

            (Some(ClassifiedCommand::External(left)), None) => {
                run_external_command(left, ctx, input, scope, true).await?
            }

            (Some(ClassifiedCommand::External(left)), _) => {
                run_external_command(left, ctx, input, scope, false).await?
            }

            (None, _) => break,
        };
    }

    Ok(input)
}
