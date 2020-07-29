use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

pub struct Url;

#[async_trait]
impl WholeStreamCommand for Url {
    fn name(&self) -> &str {
        "url"
    }

    fn signature(&self) -> Signature {
        Signature::build("url")
    }

    fn usage(&self) -> &str {
        "Apply url function"
    }

    async fn run(
        &self,
        _args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let registry = registry.clone();

        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(crate::commands::help::get_help(&Url, &registry))
                .into_value(Tag::unknown()),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::Url;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Url {})
    }
}
