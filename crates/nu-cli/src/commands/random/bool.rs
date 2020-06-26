use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use rand::prelude::{thread_rng, Rng};

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "random bool"
    }

    fn signature(&self) -> Signature {
        Signature::build("random bool")
    }

    fn usage(&self) -> &str {
        "Generate a random boolean value"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        uuid(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Generate a random boolean value",
            example: "random bool",
            result: None,
        }]
    }
}

pub async fn uuid(
    _args: CommandArgs,
    _registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let mut rng = thread_rng();
    let bool_result: bool = rng.gen_bool(0.5);
    let bool_untagged_value = UntaggedValue::boolean(bool_result);

    Ok(OutputStream::one(ReturnSuccess::value(bool_untagged_value)))
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
