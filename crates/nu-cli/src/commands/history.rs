use crate::cli::History as HistoryFile;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use std::fs::File;
use std::io::{BufRead, BufReader};

pub struct History;

#[async_trait]
impl WholeStreamCommand for History {
    fn name(&self) -> &str {
        "history"
    }

    fn signature(&self) -> Signature {
        Signature::build("history")
    }

    fn usage(&self) -> &str {
        "Display command history."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        history(args, registry)
    }
}

fn history(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let tag = args.call_info.name_tag;
    let stream = async_stream! {
        let history_path = HistoryFile::path();
        let file = File::open(history_path);
        if let Ok(file) = file {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if let Ok(line) = line {
                    yield ReturnSuccess::value(UntaggedValue::string(line).into_value(tag.clone()));
                }
            }
        } else {
            yield Err(ShellError::labeled_error("Could not open history", "history file could not be opened", tag.clone()));
        }
    };
    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::History;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(History {})
    }
}
