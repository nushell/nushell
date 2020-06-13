use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};

pub struct From;

#[async_trait]
impl WholeStreamCommand for From {
    fn name(&self) -> &str {
        "from"
    }

    fn signature(&self) -> Signature {
        Signature::build("from")
    }

    fn usage(&self) -> &str {
        "Parse content (string or binary) as a table (input format based on subcommand, like csv, ini, json, toml)"
    }

    async fn run(
        &self,
        _args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let registry = registry.clone();
        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(crate::commands::help::get_help(&From, &registry))
                .into_value(Tag::unknown()),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::From;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(From {})
    }
}
