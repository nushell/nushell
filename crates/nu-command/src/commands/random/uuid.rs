use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature};
use uuid_crate::Uuid;

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "random uuid"
    }

    fn signature(&self) -> Signature {
        Signature::build("random uuid")
    }

    fn usage(&self) -> &str {
        "Generate a random uuid4 string"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        uuid(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Generate a random uuid4 string",
            example: "random uuid",
            result: None,
        }]
    }
}

pub async fn uuid(_args: CommandArgs) -> Result<OutputStream, ShellError> {
    let uuid_4 = Uuid::new_v4().to_hyphenated().to_string();

    Ok(OutputStream::one(ReturnSuccess::value(uuid_4)))
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
