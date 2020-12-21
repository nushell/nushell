use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_data::config::{Conf, NuConfig};
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

const DEFAULT_LOCATION: &str = "history.txt";

pub fn history_path(config: &dyn Conf) -> PathBuf {
    let default_path = nu_data::config::user_data()
        .map(|mut p| {
            p.push(DEFAULT_LOCATION);
            p
        })
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_LOCATION));

    config
        .var("history-path")
        .map_or(default_path.clone(), |custom_path| {
            match custom_path.as_string() {
                Ok(path) => PathBuf::from(path),
                Err(_) => default_path,
            }
        })
}

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

        Ok(test_examples(History {})?)
    }
}
