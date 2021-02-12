use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, Value};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "split chars"
    }

    fn signature(&self) -> Signature {
        Signature::build("split chars")
    }

    fn usage(&self) -> &str {
        "splits a string's characters into separate rows"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        split_chars(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Split the string's characters into separate rows",
            example: "echo 'hello' | split chars",
            result: Some(vec![
                Value::from("h"),
                Value::from("e"),
                Value::from("l"),
                Value::from("l"),
                Value::from("o"),
            ]),
        }]
    }
}

async fn split_chars(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name = args.call_info.name_tag.clone();
    let input = args.input;
    Ok(input
        .flat_map(move |v| {
            if let Ok(s) = v.as_string() {
                futures::stream::iter(
                    s.chars()
                        .collect::<Vec<_>>()
                        .into_iter()
                        .map(move |x| ReturnSuccess::value(Value::from(x.to_string()))),
                )
                .to_output_stream()
            } else {
                OutputStream::one(Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name.span,
                    "value originates from here",
                    v.tag.span,
                )))
            }
        })
        .to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
