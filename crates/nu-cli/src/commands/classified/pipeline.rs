use crate::commands::classified::external::run_external_command;
use crate::commands::classified::internal::run_internal_command;
use crate::context::Context;
use crate::stream::InputStream;
use nu_errors::ShellError;
use nu_parser::{ClassifiedCommand, ClassifiedPipeline};
use nu_source::Text;

pub(crate) async fn run_pipeline(
    pipeline: ClassifiedPipeline,
    ctx: &mut Context,
    mut input: Option<InputStream>,
    line: &str,
) -> Result<Option<InputStream>, ShellError> {
    let mut iter = pipeline.commands.list.into_iter().peekable();

    loop {
        let item: Option<ClassifiedCommand> = iter.next();
        let next: Option<&ClassifiedCommand> = iter.peek();

        input = match (item, next) {
            (Some(ClassifiedCommand::Dynamic(_)), _) | (_, Some(ClassifiedCommand::Dynamic(_))) => {
                return Err(ShellError::unimplemented("Dynamic commands"))
            }

            (Some(ClassifiedCommand::Expr(_)), _) | (_, Some(ClassifiedCommand::Expr(_))) => {
                return Err(ShellError::unimplemented("Expression-only commands"))
            }

            (Some(ClassifiedCommand::Error(err)), _) => return Err(err.into()),
            (_, Some(ClassifiedCommand::Error(err))) => return Err(err.clone().into()),

            (Some(ClassifiedCommand::Internal(left)), _) => {
                run_internal_command(left, ctx, input, Text::from(line))?
            }

            (Some(ClassifiedCommand::External(left)), None) => {
                run_external_command(left, ctx, input, true)?
            }

            (Some(ClassifiedCommand::External(left)), _) => {
                run_external_command(left, ctx, input, false)?
            }

            (None, _) => break,
        };
    }

    Ok(input)
}
