use crate::commands::WholeStreamCommand;
use crate::data::Value;
use crate::errors::ShellError;
use crate::prelude::*;
use futures::StreamExt;
use futures_util::pin_mut;

pub struct What;

#[derive(Deserialize)]
pub struct WhatArgs {}

impl WholeStreamCommand for What {
    fn name(&self) -> &str {
        "what?"
    }

    fn signature(&self) -> Signature {
        Signature::build("what?")
    }

    fn usage(&self) -> &str {
        "Describes the objects in the stream."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, what)?.run()
    }
}

pub fn what(
    WhatArgs {}: WhatArgs,
    RunnableContext { input, host, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let values = input.values;
        pin_mut!(values);

        while let Some(row) = values.next().await {
            let name = row.format_type(host.clone().lock().unwrap().width());
            yield ReturnSuccess::value(Value::string(name).tagged(Tag::unknown_anchor(row.tag.span)));
        }
    };

    let stream: BoxStream<'static, ReturnValue> = stream.boxed();

    Ok(OutputStream::from(stream))
}
