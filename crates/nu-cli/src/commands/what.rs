use crate::commands::WholeStreamCommand;
use crate::prelude::*;

use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, ReturnValue, Signature, UntaggedValue};

pub struct What;

#[derive(Deserialize)]
pub struct WhatArgs {}

#[async_trait]
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        // args.process(registry, what)?.run()
        what(args, registry)
    }
}

pub fn what(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let mut input = args.input;
        while let Some(row) = input.next().await {
            let name = value::format_type(&row, 100);
            yield ReturnSuccess::value(UntaggedValue::string(name).into_value(Tag::unknown_anchor(row.tag.span)));
        }
    };

    let stream: BoxStream<'static, ReturnValue> = stream.boxed();

    Ok(OutputStream::from(stream))
}

#[cfg(test)]
mod tests {
    use super::What;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(What {})
    }
}
