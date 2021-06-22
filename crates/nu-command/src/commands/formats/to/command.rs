use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};

#[derive(Clone)]
pub struct To;

impl WholeStreamCommand for To {
    fn name(&self) -> &str {
        "to"
    }

    fn signature(&self) -> Signature {
        Signature::build("to")
    }

    fn usage(&self) -> &str {
        "Convert table into an output format (based on subcommand, like csv, html, json, yaml)."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(
            UntaggedValue::string(get_full_help(&To, args.scope())).into_value(Tag::unknown()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::To;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(To {})
    }
}
