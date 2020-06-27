use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue, Value};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str collect"
    }

    fn signature(&self) -> Signature {
        Signature::build("str collect")
    }

    fn usage(&self) -> &str {
        "collects a list of strings into a string"
    }

    async fn run(
        &self,
        args: CommandArgs,
        _registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let output = args
            .input
            .collect_string(args.call_info.name_tag.clone())
            .await?;

        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(output.item).into_value(output.tag),
        )))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Collect a list of string",
            example: "echo ['a' 'b' 'c'] | str collect",
            result: Some(vec![Value::from("abc")]),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
