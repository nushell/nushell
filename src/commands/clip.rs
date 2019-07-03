use crate::commands::command::SinkCommandArgs;
use crate::errors::{labelled, ShellError};
use clipboard::{ClipboardContext, ClipboardProvider};

pub fn clip(args: SinkCommandArgs) -> Result<(), ShellError> {
    let mut clip_context: ClipboardContext = ClipboardProvider::new().unwrap();
    let mut new_copy_data = String::new();

    if args.input.len() > 0 {
        let mut first = true;
        for i in args.input.iter() {
            if !first {
                new_copy_data.push_str("\n");
            } else {
                first = false;
            }

            let string = i.as_string().map_err(labelled(
                args.name_span,
                "Given non-string data",
                "expected strings from pipeline",
            ))?;

            new_copy_data.push_str(&string);
        }
    }

    clip_context.set_contents(new_copy_data).unwrap();

    Ok(())
}
