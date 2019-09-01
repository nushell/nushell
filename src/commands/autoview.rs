use crate::commands::{RawCommandArgs, WholeStreamCommand};
use crate::errors::ShellError;
use crate::prelude::*;

pub struct Autoview;

#[derive(Deserialize)]
pub struct AutoviewArgs {}

impl WholeStreamCommand for Autoview {
    fn name(&self) -> &str {
        "autoview"
    }

    fn signature(&self) -> Signature {
        Signature::build("autoview")
    }

    fn usage(&self) -> &str {
        "View the contents of the pipeline as a table or list."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        Ok(args.process_raw(registry, autoview)?.run())
    }
}

pub fn autoview(
    AutoviewArgs {}: AutoviewArgs,
    mut context: RunnableContext,
    raw: RawCommandArgs,
) -> Result<OutputStream, ShellError> {
    Ok(OutputStream::new(async_stream_block! {
        let input = context.input.drain_vec().await;

        if input.len() > 0 {
            if let Tagged {
                item: Value::Binary(_),
                ..
            } = input[0usize]
            {
                let binary = context.expect_command("binaryview");
                let result = binary.run(raw.with_input(input), &context.commands);
                result.collect::<Vec<_>>().await;
            } else if is_single_text_value(&input) {
                let text = context.expect_command("textview");
                let result = text.run(raw.with_input(input), &context.commands);
                result.collect::<Vec<_>>().await;
            } else if equal_shapes(&input) {
                let table = context.expect_command("table");
                let result = table.run(raw.with_input(input), &context.commands);
                result.collect::<Vec<_>>().await;
            } else {
                let table = context.expect_command("table");
                let result = table.run(raw.with_input(input), &context.commands);
                result.collect::<Vec<_>>().await;
            }
        }
    }))
}

fn equal_shapes(input: &Vec<Tagged<Value>>) -> bool {
    let mut items = input.iter();

    let item = match items.next() {
        Some(item) => item,
        None => return false,
    };

    let desc = item.data_descriptors();

    for item in items {
        if desc != item.data_descriptors() {
            return false;
        }
    }

    true
}

fn is_single_text_value(input: &Vec<Tagged<Value>>) -> bool {
    if input.len() != 1 {
        return false;
    }
    if let Tagged {
        item: Value::Primitive(Primitive::String(_)),
        ..
    } = input[0]
    {
        true
    } else {
        false
    }
}
