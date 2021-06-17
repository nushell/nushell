use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use std::fs::File;
use std::io::{BufRead, BufReader};

pub struct History;

impl WholeStreamCommand for History {
    fn name(&self) -> &str {
        "history"
    }

    fn signature(&self) -> Signature {
        Signature::build("history").switch("clear", "Clears out the history entries", Some('c'))
    }

    fn usage(&self) -> &str {
        "Display command history."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        history(args)
    }
}

fn history(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let tag = args.call_info.name_tag.clone();
    let ctx = &args.context;

    let clear = args.has_flag("clear");

    let path = if let Some(global_cfg) = &ctx.configs().lock().global_config {
        nu_data::config::path::history_path_or_default(global_cfg)
    } else {
        nu_data::config::path::default_history_path()
    };

    if clear {
        // This is a NOOP, the logic to clear is handled in cli.rs
        Ok(ActionStream::empty())
    } else if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        // Skips the first line, which is a Rustyline internal
        let output = reader.lines().skip(1).filter_map(move |line| match line {
            Ok(line) => Some(ReturnSuccess::value(
                UntaggedValue::string(line).into_value(tag.clone()),
            )),
            Err(_) => None,
        });

        Ok(output.into_action_stream())
    } else {
        Err(ShellError::labeled_error(
            "Could not open history",
            "history file could not be opened",
            tag,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::History;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(History {})
    }
}
