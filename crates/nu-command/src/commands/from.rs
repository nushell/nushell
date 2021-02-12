use crate::prelude::*;
use nu_engine::WholeStreamCommand;
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(get_help(&From, &args.scope)).into_value(Tag::unknown()),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::From;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(From {})
    }
}
