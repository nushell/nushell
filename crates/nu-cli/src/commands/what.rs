use crate::commands::WholeStreamCommand;
use crate::prelude::*;

use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, ReturnValue, Signature, UntaggedValue};

pub struct What;

#[derive(Deserialize)]
pub struct WhatArgs {}

impl WholeStreamCommand for What {
    fn name(&self) -> &str {
        "describe"
    }

    fn signature(&self) -> Signature {
        Signature::build("describe")
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
    _: WhatArgs,
    RunnableContext { mut input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        while let Some(row) = input.next().await {
            let name = value::format_type(&row, 100);
            yield ReturnSuccess::value(UntaggedValue::string(name).into_value(Tag::unknown_anchor(row.tag.span)));
        }
    };

    let stream: BoxStream<'static, ReturnValue> = stream.boxed();

    Ok(OutputStream::from(stream))
}
