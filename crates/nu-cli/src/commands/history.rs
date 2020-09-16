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
    let config: Box<dyn Conf> = Box::new(NuConfig::new());
    let tag = args.call_info.name_tag;
    let path = history_path(&config);
    let file = File::open(path);
    if let Ok(file) = file {
        let reader = BufReader::new(file);
        let output = reader.lines().filter_map(move |line| match line {
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

#[cfg(test)]
mod tests {
    use super::History;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(History {})
    }
}
