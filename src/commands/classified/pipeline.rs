use crate::commands::classified::external::{run_external_command, StreamNext};
use crate::commands::classified::internal::run_internal_command;
use crate::commands::classified::ClassifiedInputStream;
use crate::context::Context;
use crate::stream::OutputStream;
use nu_errors::ShellError;
use nu_parser::{ClassifiedCommand, ClassifiedPipeline};
use nu_protocol::{ReturnSuccess, UntaggedValue, Value};
use nu_source::Text;
use std::sync::atomic::Ordering;

pub(crate) async fn run_pipeline(
    pipeline: ClassifiedPipeline,
    ctx: &mut Context,
    mut input: ClassifiedInputStream,
    line: &str,
) -> Result<(), ShellError> {
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
                let stream = run_internal_command(left, ctx, input, Text::from(line)).await?;
                ClassifiedInputStream::from_input_stream(stream)
            }

            (Some(ClassifiedCommand::External(left)), Some(ClassifiedCommand::External(_))) => {
                run_external_command(left, ctx, input, StreamNext::External).await?
            }

            (Some(ClassifiedCommand::External(left)), Some(_)) => {
                run_external_command(left, ctx, input, StreamNext::Internal).await?
            }

            (Some(ClassifiedCommand::External(left)), None) => {
                run_external_command(left, ctx, input, StreamNext::Last).await?
            }

            (None, _) => break,
        };
    }

    use futures::stream::TryStreamExt;
    let mut output_stream: OutputStream = input.objects.into();
    loop {
        match output_stream.try_next().await {
            Ok(Some(ReturnSuccess::Value(Value {
                value: UntaggedValue::Error(e),
                ..
            }))) => return Err(e),
            Ok(Some(_item)) => {
                if ctx.ctrl_c.load(Ordering::SeqCst) {
                    break;
                }
            }
            _ => {
                break;
            }
        }
    }

    Ok(())
}
