use crate::prelude::*;
use nu_data::config::{Conf, NuConfig};
use nu_engine::history_path;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Deserialize)]
struct Arguments {
    clear: Option<bool>,
}

pub struct History;

#[async_trait]
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        history(args).await
    }
}

async fn history(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let config: Box<dyn Conf> = Box::new(NuConfig::new());
    let tag = args.call_info.name_tag.clone();
    let (Arguments { clear }, _) = args.process().await?;

    let path = history_path(&config);

    match clear {
        Some(_) => {
            // This is a NOOP, the logic to clear is handled in cli.rs
            Ok(OutputStream::empty())
        }
        None => {
            if let Ok(file) = File::open(path) {
                let reader = BufReader::new(file);
                // Skips the first line, which is a Rustyline internal
                let output = reader.lines().skip(1).filter_map(move |line| match line {
                    Ok(line) => Some(ReturnSuccess::value(
                        UntaggedValue::string(line).into_value(tag.clone()),
                    )),
                    Err(_) => None,
                });

                Ok(futures::stream::iter(output).to_output_stream())
            } else {
                Err(ShellError::labeled_error(
                    "Could not open history",
                    "history file could not be opened",
                    tag,
                ))
            }
        }
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
