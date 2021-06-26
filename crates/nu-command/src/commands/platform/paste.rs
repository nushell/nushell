use crate::prelude::*;

use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue};

use arboard::Clipboard;

pub struct Paste;

impl WholeStreamCommand for Paste {
    fn name(&self) -> &str {
        "paste"
    }

    fn signature(&self) -> Signature {
        Signature::build("paste")
    }

    fn usage(&self) -> &str {
        "Paste contents from the clipboard"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        paste(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Paste text from your clipboard",
            example: "echo 'secret value' | clip | paste",
            result: Some(vec![UntaggedValue::Primitive(Primitive::String(
                "secret value".to_owned(),
            ))
            .into_value(Tag::default())]),
        }]
    }
}

pub fn paste(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let name = args.call_info.name_tag;

    if let Ok(mut clip_context) = Clipboard::new() {
        match clip_context.get_text() {
            Ok(out) => Ok(ActionStream::one(ReturnSuccess::value(
                UntaggedValue::Primitive(Primitive::String(out)),
            ))),
            Err(_) => Err(ShellError::labeled_error(
                "Could not get contents of clipboard",
                "could not get contents of clipboard",
                name,
            )),
        }
    } else {
        Err(ShellError::labeled_error(
            "Could not open clipboard",
            "could not open clipboard",
            name,
        ))
    }
}
