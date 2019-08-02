use crate::commands::{CommandArgs, StaticCommand};
use crate::context::CommandRegistry;
use crate::errors::{labelled, ShellError};
use crate::prelude::*;
use clipboard::{ClipboardContext, ClipboardProvider};
use futures::stream::StreamExt;
use futures_async_stream::async_stream_block;

pub struct Clip;

#[derive(Deserialize)]
pub struct ClipArgs {}

impl StaticCommand for Clip {
    fn name(&self) -> &str {
        "clip"
    }
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, clip)?.run()
    }

    fn signature(&self) -> Signature {
        Signature::build("clip").sink()
    }
}

pub fn clip(
    ClipArgs {}: ClipArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream_block! {
        let values: Vec<Spanned<Value>> = input.values.collect().await;

        inner_clip(values, name);
    };

    let stream: BoxStream<'static, ReturnValue> = stream.boxed();

    Ok(OutputStream::from(stream))
}

async fn inner_clip(input: Vec<Spanned<Value>>, name: Option<Span>) -> OutputStream {
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

            let s = i.as_string().map_err(labelled(
                name,
                "Given non-string data",
                "expected strings from pipeline",
            ));

            let string: String = match s {
                Ok(string) => string,
                Err(err) => return OutputStream::one(Err(err)),
            };

            new_copy_data.push_str(&string);
        }
    }

    clip_context.set_contents(new_copy_data).unwrap();

    OutputStream::empty()
}
