use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use chrono::{TimeZone, Utc};
use nu_data::config::{Conf, NuConfig};
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue, Value};
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

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "History of commands",
                example: "history",
                result: None,
            },
            Example {
                description: "Commands within last day",
                example: "history | where timestamp < 1day",
                result: None,
            },
            Example {
                description: "Clear out history entries",
                example: "history --clear",
                result: None,
            },
        ]
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        history(args, registry).await
    }
}

async fn history(
    args: CommandArgs,
    _registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let config: Box<dyn Conf> = Box::new(NuConfig::new());
    let tag = args.call_info.name_tag.clone();
    let (Arguments { clear }, _) = args.process(&_registry).await?;

    let path = history_path(&config);

    match clear {
        Some(_) => {
            // This is a NOOP, the logic to clear is handled in cli.rs
            Ok(OutputStream::empty())
        }
        None => {
            if let Ok(file) = File::open(path) {
                let mut prev_was_timestamp = false;
                let mut prev_timestamp = "".to_string();
                let mut rows = VecDeque::new();

                // Skips the first line, which is a Rustyline internal
                for line in BufReader::new(&file).lines().skip(1) {
                    let mut map = IndexMap::<String, Value>::new();
                    let current_line = line?.clone();

                    // The previous line was a timestamp for the current command
                    if prev_was_timestamp {
                        match prev_timestamp.parse::<i64>() {
                            Ok(ts) => {
                                map.insert(
                                    "timestamp".to_string(),
                                    UntaggedValue::date(Utc.timestamp(ts, 0)).into_untagged_value(),
                                );
                                map.insert(
                                    "command".to_string(),
                                    UntaggedValue::string(current_line).into_untagged_value(),
                                );
                            }
                            Err(_) => {
                                // Malformed timestamp found for this command, use current time as default
                                map.insert(
                                    "timestamp".to_string(),
                                    UntaggedValue::date(chrono::Utc::now()).into_untagged_value(),
                                );
                                map.insert(
                                    "command".to_string(),
                                    UntaggedValue::string(current_line).into_untagged_value(),
                                );
                            }
                        };
                        prev_was_timestamp = false;
                    } else {
                        // Set the timestamp, if found
                        if current_line.starts_with('#') {
                            prev_timestamp = current_line.trim_start_matches('#').to_string();
                            prev_was_timestamp = true;
                            continue;
                        } else {
                            // No timestamp was found for this command, use current time as default
                            map.insert(
                                "timestamp".to_string(),
                                UntaggedValue::date(chrono::Utc::now()).into_untagged_value(),
                            );
                            map.insert(
                                "command".to_string(),
                                UntaggedValue::string(current_line).into_untagged_value(),
                            );
                        }
                    }

                    rows.push_back(UntaggedValue::row(map).into_value(&tag));
                }
                Ok(futures::stream::iter(rows).to_output_stream())
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
