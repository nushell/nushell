use crate::prelude::*;

use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, Value};

use arboard::Clipboard;

pub struct Clip;

impl WholeStreamCommand for Clip {
    fn name(&self) -> &str {
        "clip"
    }

    fn signature(&self) -> Signature {
        Signature::build("clip")
    }

    fn usage(&self) -> &str {
        "Copy the contents of the pipeline to the copy/paste buffer."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        clip(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Save text to the clipboard",
                example: "echo 'secret value' | clip",
                result: None,
            },
            Example {
                description: "Save numbers to the clipboard",
                example: "random integer 10000000..99999999 | clip",
                result: None,
            },
        ]
    }
}

pub fn clip(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let input = args.input;
    let name = args.call_info.name_tag;
    let values: Vec<Value> = input.collect();

    if let Ok(mut clip_context) = Clipboard::new() {
        let mut new_copy_data = String::new();

        if !values.is_empty() {
            let mut first = true;
            for i in values.iter() {
                if !first {
                    new_copy_data.push('\n');
                } else {
                    first = false;
                }

                let string: String = i.convert_to_string();
                if string.is_empty() {
                    return Err(ShellError::labeled_error(
                        "Unable to convert to string",
                        "Unable to convert to string",
                        name,
                    ));
                }

                new_copy_data.push_str(&string);
            }
        }

        match clip_context.set_text(new_copy_data) {
            Ok(_) => {}
            Err(_) => {
                return Err(ShellError::labeled_error(
                    "Could not set contents of clipboard",
                    "could not set contents of clipboard",
                    name,
                ));
            }
        }
    } else {
        return Err(ShellError::labeled_error(
            "Could not open clipboard",
            "could not open clipboard",
            name,
        ));
    }
    Ok(ActionStream::empty())
}

#[cfg(test)]
mod tests {
    use super::Clip;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Clip {})
    }
}
