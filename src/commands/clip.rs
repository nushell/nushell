#[cfg(feature = "clipboard")]
pub mod clipboard {
    use crate::commands::WholeStreamCommand;
    use crate::context::CommandRegistry;
    use crate::errors::ShellError;
    use crate::prelude::*;
    use futures::stream::StreamExt;
    use futures_async_stream::async_stream_block;

    use clipboard::{ClipboardContext, ClipboardProvider};

    pub struct Clip;

    #[derive(Deserialize)]
    pub struct ClipArgs {}

    impl WholeStreamCommand for Clip {
        fn name(&self) -> &str {
            "clip"
        }

        fn signature(&self) -> Signature {
            Signature::build("clip")
        }

        fn usage(&self) -> &str {
            "Copy the contents of the pipeline to the copy/paste buffer"
        }

        fn run(
            &self,
            args: CommandArgs,
            registry: &CommandRegistry,
        ) -> Result<OutputStream, ShellError> {
            args.process(registry, clip)?.run()
        }
    }

    pub fn clip(
        ClipArgs {}: ClipArgs,
        RunnableContext { input, name, .. }: RunnableContext,
    ) -> Result<OutputStream, ShellError> {
        let stream = async_stream_block! {
            let values: Vec<Tagged<Value>> = input.values.collect().await;

            inner_clip(values, name).await;
        };

        let stream: BoxStream<'static, ReturnValue> = stream.boxed();

        Ok(OutputStream::from(stream))
    }

    async fn inner_clip(input: Vec<Tagged<Value>>, name: Span) -> OutputStream {
        let mut clip_context: ClipboardContext = ClipboardProvider::new().unwrap();
        let mut new_copy_data = String::new();

        if input.len() > 0 {
            let mut first = true;
            for i in input.iter() {
                if !first {
                    new_copy_data.push_str("\n");
                } else {
                    first = false;
                }

                let string: String = match i.as_string() {
                    Ok(string) => string,
                    Err(_) => {
                        return OutputStream::one(Err(ShellError::labeled_error(
                            "Given non-string data",
                            "expected strings from pipeline",
                            name,
                        )))
                    }
                };

                new_copy_data.push_str(&string);
            }
        }

        clip_context.set_contents(new_copy_data).unwrap();

        OutputStream::empty()
    }
}
