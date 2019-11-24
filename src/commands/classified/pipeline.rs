use super::{ClassifiedCommand, ClassifiedInputStream, StreamNext};
use crate::prelude::*;
use std::fmt;
use std::sync::atomic::Ordering;

#[derive(Debug, Clone)]
pub(crate) struct Pipeline {
    pub(crate) commands: Spanned<Vec<ClassifiedCommand>>,
}

impl FormatDebug for Pipeline {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        f.say_str(
            "classified pipeline",
            self.commands.iter().map(|c| c.debug(source)).join(" | "),
        )
    }
}

impl HasSpan for Pipeline {
    fn span(&self) -> Span {
        self.commands.span
    }
}

impl Pipeline {
    pub(crate) async fn run(
        self,
        ctx: &mut Context,
        mut input: ClassifiedInputStream,
        line: &str,
    ) -> Result<(), ShellError> {
        let mut iter = self.commands.item.into_iter().peekable();

        loop {
            let item: Option<ClassifiedCommand> = iter.next();
            let next: Option<&ClassifiedCommand> = iter.peek();

            input = match (item, next) {
                (Some(ClassifiedCommand::Dynamic(_)), _)
                | (_, Some(ClassifiedCommand::Dynamic(_))) => {
                    return Err(ShellError::unimplemented("Dynamic commands"))
                }

                (Some(ClassifiedCommand::Expr(_)), _) | (_, Some(ClassifiedCommand::Expr(_))) => {
                    return Err(ShellError::unimplemented("Expression-only commands"))
                }

                (Some(ClassifiedCommand::Internal(left)), _) => {
                    let stream = left.run(ctx, input, Text::from(line))?;
                    ClassifiedInputStream::from_input_stream(stream)
                }

                (Some(ClassifiedCommand::External(left)), Some(ClassifiedCommand::External(_))) => {
                    left.run(ctx, input, StreamNext::External).await?
                }

                (Some(ClassifiedCommand::External(left)), Some(_)) => {
                    left.run(ctx, input, StreamNext::Internal).await?
                }

                (Some(ClassifiedCommand::External(left)), None) => {
                    left.run(ctx, input, StreamNext::Last).await?
                }

                (None, _) => break,
            };
        }

        use futures::stream::TryStreamExt;
        let mut output_stream: OutputStream = input.objects.into();
        loop {
            match output_stream.try_next().await {
                Ok(Some(ReturnSuccess::Value(Tagged {
                    item: Value::Error(e),
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
}
