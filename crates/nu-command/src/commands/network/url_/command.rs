use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};

pub struct Url;

impl WholeStreamCommand for Url {
    fn name(&self) -> &str {
        "url"
    }

    fn signature(&self) -> Signature {
        Signature::build("url")
    }

    fn usage(&self) -> &str {
        "Apply url function."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(
            UntaggedValue::string(get_full_help(&Url, args.scope())).into_value(Tag::unknown()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::Url;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Url {})
    }
}
